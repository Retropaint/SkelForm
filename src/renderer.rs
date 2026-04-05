//! Core rendering logic, abstracted from the rest of WGPU.

use crate::*;
use image::GenericImageView;
use utils::shortest_angle_delta;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};

/// The `main` of this module.
pub fn render(
    render_pass: &mut RenderPass,
    device: &Device,
    queue: &wgpu::Queue,
    camera: &Camera,
    input: &InputStates,
    armature: &Armature,
    config: &Config,
    edit_mode: &EditMode,
    selections: &SelectionState,
    renderer: &mut Renderer,
    events: &mut EventState,
) {
    if camera.window == Vec2::ZERO {
        return;
    }
    let sel = selections.clone();

    renderer.bone_buffer.init(device, 1000);
    renderer.prev_onion_buffer.init(device, 1000);
    renderer.next_onion_buffer.init(device, 1000);
    renderer.point_buffer.init(device, 1000);
    renderer.kite_buffer.init(device, 1000);
    renderer.sel_bone_buffer.init(device, 1000);
    renderer.gridline_buffer.init(device, 1000);
    renderer.meshframe_buffer.init(device, 1000);
    renderer.ring_buffer.init(device, 1000);
    renderer.selected_ring_buffer.init(device, 10);
    renderer.rect_buffer.init(device, 1000);

    // no edits are being made if the LMB isn't down
    if !input.left_down && (edit_mode.is_moving || edit_mode.is_rotating || edit_mode.is_scaling) {
        events.update_current_editing(1);
    }

    // inform HTML if canvas has successfully loaded
    #[cfg(target_arch = "wasm32")]
    if !renderer.has_loaded {
        loaded();
        renderer.has_loaded = true;
    }

    // create vert on cursor
    let space = utils::screen_to_world_space(input.mouse, camera.window);
    let mut mouse_world_vert = vert(Some(space), None, None);
    mouse_world_vert.pos.x *= camera.window.y / camera.window.x;

    // mouse pos in world space
    let mouse_pos = Vec2::new(
        mouse_world_vert.pos.x * camera.zoom / camera.aspect_ratio() + camera.pos.x,
        mouse_world_vert.pos.y * camera.zoom + camera.pos.y,
    );

    if !config.gridline_front {
        draw_gridline(render_pass, renderer, &camera, &config, queue);
    }

    // turn off hovering vert if not editing mesh
    if !edit_mode.showing_mesh && selections.hovering_vert_id != -1 {
        events.set_hovering_id(-1);
    }

    // temporary armature, to be used for rendering
    let mut temp_arm = Armature::default();
    let mut anim_bones = armature.animated_bones.clone();

    // adjust anim_bones' verts for new textrues mid-animations
    temp_arm.bones = armature.animated_bones.clone();
    for b in 0..armature.bones.len() {
        let tex = temp_arm.tex_of(armature.bones[b].id);
        if !armature.bones[b].verts_edited && tex != None {
            let size = tex.unwrap().size;
            (anim_bones[b].vertices, anim_bones[b].indices) = create_tex_rect(&size);
        }
    }
    temp_arm.bones = renderer.temp_bones.clone();

    // animate next and previous frame armatures for onions
    let mut next_arm = Armature::default();
    let mut prev_arm = Armature::default();
    if selections.anim_frame != -1 && edit_mode.onion_layers {
        let keyframes = &armature.sel_anim(selections).unwrap().keyframes;

        // set up previous onion
        prev_arm.bones = armature.bones.clone();
        prev_arm.animations = armature.animations.clone();
        let mut prev_sel = selections.clone();
        let idx = keyframes.iter().position(|kf| kf.frame > sel.anim_frame);
        prev_sel.anim_frame = keyframes[idx.unwrap_or_else(|| 1) - 1].frame;
        utils::animate_bones(&mut prev_arm, &prev_sel, edit_mode);
        prev_arm.bones = prev_arm.animated_bones.clone();
        construction(&mut prev_arm.bones, &prev_arm.animated_bones);

        // set up next onion
        next_arm.bones = armature.bones.clone();
        next_arm.animations = armature.animations.clone();
        let mut next_sel = prev_sel.clone();
        if let Some(next_kf) = keyframes.iter().find(|kf| kf.frame > selections.anim_frame) {
            next_sel.anim_frame = next_kf.frame;
        }
        utils::animate_bones(&mut next_arm, &next_sel, edit_mode);
        next_arm.bones = next_arm.animated_bones.clone();
        construction(&mut next_arm.bones, &next_arm.animated_bones);
    }

    // get all children of selected bone(s)
    let mut selected_bone_ids = vec![];
    if armature.sel_bone(&sel) != None {
        for id in &selections.bone_ids {
            let bone = temp_arm.bones.iter().find(|bone| bone.id == *id).unwrap();
            let mut children = vec![bone.clone()];
            armature_window::get_all_children(&temp_arm.bones, &mut children, &bone);
            for child in children {
                selected_bone_ids.push(child.id);
            }
        }
    }

    // setup propagated group colors
    for b in 0..temp_arm.bones.len() {
        if temp_arm.bones[b].group_color.a != 0 {
            continue;
        }
        let parent_id = &temp_arm.bones[b].parent_id;
        let parent = temp_arm.bones.iter().find(|b| b.id == *parent_id);
        if parent != None {
            temp_arm.bones[b].group_color = parent.unwrap().group_color;
        }
    }

    // sort bones by highest zindex first, so that hover logic will pick the top-most one
    temp_arm.bones.sort_by(|a, b| b.zindex.cmp(&a.zindex));
    prev_arm.bones.sort_by(|a, b| b.zindex.cmp(&a.zindex));
    next_arm.bones.sort_by(|a, b| b.zindex.cmp(&a.zindex));

    let mut hover_bone_id = -1;

    // many fight for spot of newest vertex; only one will emerge victorious.
    let mut new_vert: Option<Vertex> = None;
    let mut hovered_vert = false;

    // pre-draw bone setup
    for b in 0..temp_arm.bones.len() {
        let tex = armature.tex_of(temp_arm.bones[b].id);
        let parents = armature.get_all_parents(false, temp_arm.bones[b].id);

        if tex == None
            || temp_arm.is_bone_hidden(false, config.propagate_visibility, temp_arm.bones[b].id)
        {
            continue;
        }

        // setup world verts
        let cam = world_camera(&camera, &config);
        for v in 0..temp_arm.bones[b].vertices.len() {
            let tb = &mut temp_arm.bones[b];
            let mut vert = world_vert(tb.vertices[v], &cam, camera.aspect_ratio(), Vec2::default());
            vert.tint = tb.tint;
            tb.world_verts.push(vert);
        }

        // setup onion world verts
        if selections.anim_frame != -1 && edit_mode.onion_layers {
            macro_rules! prep_arm {
                ($armature:expr, $color:expr) => {
                    for v in 0..$armature.bones[b].vertices.len() {
                        let tb = &mut $armature.bones[b];
                        let ratio = camera.aspect_ratio();
                        let mut vert = world_vert(tb.vertices[v], &camera, ratio, Vec2::default());
                        vert.tint = TintColor::new(255., 0., 0., 0.4);
                        tb.world_verts.push(vert);
                    }
                };
            }
            prep_arm!(prev_arm, TintColor::new(255., 0., 0., 0.4));
            prep_arm!(next_arm, TintColor::new(0., 0., 255., 0.4));
        }
        for vert in &mut temp_arm.bones[b].world_verts {
            vert.add_color = Color::new(0, 0, 0, 0);
        }

        // check if cursor is on an opaque pixel of this bone's texture
        let tb = &temp_arm.bones[b];
        let selected_mesh = !edit_mode.showing_mesh
            || edit_mode.showing_mesh
                && sel.bone_idx != usize::MAX
                && armature.sel_bone(&sel).unwrap().id == tb.id;
        if hover_bone_id == -1
            && (!input.left_down || input.left_pressed) // allow detection on LMB press
            && !camera.on_ui
            && selected_mesh
            && renderer.render_textures
        {
            let wv = &temp_arm.bones[b].world_verts;
            for (_, chunk) in temp_arm.bones[b].indices.chunks_exact(3).enumerate() {
                let c0 = chunk[0] as usize;
                let c1 = chunk[1] as usize;
                let c2 = chunk[2] as usize;

                let bary = tri_point(&mouse_world_vert.pos, &wv[c0].pos, &wv[c1].pos, &wv[c2].pos);
                if bary.0 == -1. {
                    continue;
                }

                // initiate vertex position
                let bones = &temp_arm.bones;
                let v = &bones.iter().find(|bone| bone.id == tb.id).unwrap().vertices;
                let uv = v[c0].uv * bary.3 + v[c1].uv * bary.1 + v[c2].uv * bary.2;
                let mut pos = (utils::rotate(&(v[c0].pos - tb.pos), -tb.rot)) * bary.3
                    + (utils::rotate(&(v[c1].pos - tb.pos), -tb.rot)) * bary.1
                    + (utils::rotate(&(v[c2].pos - tb.pos), -tb.rot)) * bary.2;
                pos /= tb.scale;

                // editing this bone's mesh, add this as new vertex candidate
                if edit_mode.showing_mesh && input.left_clicked && new_vert == None {
                    new_vert = Some(vert(Some(pos), None, Some(uv)));
                    break;
                }

                // set this bone as hovered, if the cursor is on its texture
                let tex = armature.tex_of(temp_arm.bones[b].id).unwrap();
                let img = &armature.tex_data(tex).unwrap().image;
                let pos = Vec2::new(
                    (uv.x * img.width() as f32).min(img.width() as f32 - 1.),
                    (uv.y * img.height() as f32).min(img.height() as f32 - 1.),
                );
                let pixel_alpha = img.get_pixel(pos.x as u32, pos.y as u32).0[3];
                if pixel_alpha == 255 && !edit_mode.showing_mesh {
                    hover_bone_id = temp_arm.bones[b].id;
                    break;
                }
            }
        }

        let mut click_on_hover_id = temp_arm.bones[b].id;
        if !config.exact_bone_select {
            // QoL: select parent of textured bone if it's called 'Texture'
            // this is because most textured bones are meant to represent their parents
            if parents.len() != 0 && temp_arm.bones[b].name.to_lowercase() == "texture" {
                click_on_hover_id = parents[0].id;
            }
        }

        // hovering glow animation
        let idx = selections.bone_idx;
        let not_selected = idx == usize::MAX || armature.bones[idx].id != click_on_hover_id;
        if hover_bone_id == temp_arm.bones[b].id && not_selected && !renderer.on_point {
            let fade = (64. * ((edit_mode.time * 3.).sin()).abs()).min(255.);
            let min = 25;
            for vert in &mut temp_arm.bones[b].world_verts {
                vert.add_color =
                    Color::new(min + fade as u8, min + fade as u8, min + fade as u8, 0);
            }

            // select bone if clicked
            if input.left_pressed && !renderer.on_point {
                let id = click_on_hover_id;
                let bones = &armature.bones;
                let idx = bones.iter().position(|bone| bone.id == id).unwrap();
                events.select_bone(idx, true);
            }
        } else {
            for vert in &mut temp_arm.bones[b].world_verts {
                vert.add_color = Color::new(0, 0, 0, 0);
            }
        }
    }
    renderer.on_point = false;

    renderer.temp_bones = temp_arm.bones.clone();

    // runtime: sort bones by z-index for drawing
    temp_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    // sort onions by zindex as well
    if selections.anim_frame != -1 && edit_mode.onion_layers {
        prev_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));
        next_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));
    }

    // render onion layers
    if renderer.render_textures && selections.anim_frame != -1 && edit_mode.onion_layers {
        #[rustfmt::skip]
        draw_armature(&prev_arm, armature, edit_mode.showing_mesh, &sel, config, queue, render_pass, &renderer.prev_onion_buffer);
        #[rustfmt::skip]
        draw_armature(&next_arm, armature, edit_mode.showing_mesh, &sel, config, queue, render_pass, &renderer.next_onion_buffer);
    }

    // render bones
    if renderer.render_textures {
        #[rustfmt::skip]
        draw_armature(&temp_arm, armature, edit_mode.showing_mesh, &sel, config, queue, render_pass, &renderer.bone_buffer);
    }

    // show selected bone's mesh wireframe if editing it
    let mut hovering_vert_id = -1;
    let mut is_hovering_tri = false;
    let mut is_hovering_line = false;
    if edit_mode.showing_mesh && armature.sel_bone(&sel) != None {
        let id = armature.sel_bone(&sel).unwrap().id;
        let bone = temp_arm.bones.iter().find(|bone| bone.id == id).unwrap();

        // render texture, so it appears above everything else
        let tex = armature.tex_of(bone.id);
        if renderer.render_textures && tex != None{
            let bind_group = &armature.tex_data(tex.unwrap()).unwrap().bind_group;
            let sel_bone_buffer = &mut renderer.sel_bone_buffer;
            let mut world_verts = bone.world_verts.clone();
            for vert in &mut world_verts {
                vert.color = Color::new(255, 255, 255, 255);
            }
            render_pass.set_bind_group(0, bind_group, &[]);
            setup_render_buffer(sel_bone_buffer, &world_verts, &bone.indices, queue);
            draw(&sel_bone_buffer, render_pass, 0, bone.indices.len());
        }

        render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);
        let mouse = mouse_world_vert;
        let wv = bone.world_verts.clone();

        // prepare drawing buffers for vertex points and lines
        let current_hover_id = selections.hovering_vert_id;
        #[rustfmt::skip]
        let (mut verts, mut indices, on_vert) =
            bone_vertices(&wv, true, selections, input, camera, config, events, armature, renderer, current_hover_id);
        if on_vert != -1 {
            new_vert = None;
            hovering_vert_id = on_vert;
        } else {
            renderer.clicked_vert_id = -1;
        }
        #[rustfmt::skip]
        let (mut lines_v, mut lines_i, on_line) =
            vert_lines(bone, &temp_arm.bones, &mouse, &mut new_vert, true, on_vert != -1, camera, input, selections, events);
        is_hovering_line |= on_line;
        lines_v.append(&mut verts);
        add_offseted_indices(&mut indices, &mut lines_i);

        // draw hovered triangle if neither a vertex nor a line is hovered
        let (idx, mut hovering_tri) = bone_triangle(&bone, &mouse, wv);
        if hovering_tri.len() > 0 && on_vert == -1 && !on_line && !camera.on_ui {
            is_hovering_tri = true;
            hovering_tri[0].color = Color::new(0, 200, 0, 100);
            hovering_tri[1].color = Color::new(0, 200, 0, 100);
            hovering_tri[2].color = Color::new(0, 200, 0, 100);
            lines_v.append(&mut hovering_tri.clone());
            add_offseted_indices(&mut vec![0, 1, 2], &mut lines_i);

            // verts of this triangle will be dragged
            if input.left_pressed {
                events.select_vertex(hovering_tri[0].id as i32, false);
                events.select_vertex(hovering_tri[1].id as i32, true);
                events.select_vertex(hovering_tri[2].id as i32, true);
            }

            // remove this triangle if right-clicking
            if edit_mode.showing_mesh && input.right_clicked {
                if armature.sel_bone(&sel).unwrap().indices.len() == 6 {
                    events.open_modal("indices_limit", false);
                } else {
                    events.remove_triangle(idx as usize * 3);
                }
            }
        }
        hovered_vert = on_vert != -1 && !camera.on_ui;

        // draw vertex points and lines
        setup_render_buffer(&mut renderer.meshframe_buffer, &lines_v, &lines_i, queue);
        draw(&renderer.meshframe_buffer, render_pass, 0, lines_i.len());
    }

    // increment hovering tri countdown, to show tooltip on UI
    if is_hovering_tri && input.mouse == input.mouse_prev {
        events.set_hovering_tri(selections.hovering_tri_dur + 1);
    } else if selections.hovering_tri_dur != 0 {
        events.set_hovering_tri(0);
    }

    // increment hovering line countdown, to show tooltip on UI
    if is_hovering_line && input.mouse == input.mouse_prev {
        events.set_hovering_line(selections.hovering_line_dur + 1);
    } else if selections.hovering_line_dur != 0 {
        events.set_hovering_line(0);
    }

    // set hovering vert, to display ID on UI
    if hovering_vert_id != -1 {
        events.set_hovering_id(hovering_vert_id);
    }

    // draw render rects if enabled
    if renderer.render_rects {
        let mut verts = vec![];
        let mut indices = vec![];
        render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);
        for bone in &temp_arm.bones {
            if bone.verts_edited
                || temp_arm.is_bone_hidden(false, config.propagate_visibility, bone.id)
            {
                continue;
            }

            let mut world_verts = bone.world_verts.clone();

            // color wireframes based on bone group color
            for vert in &mut world_verts {
                if bone.group_color.a == 0 {
                    vert.color = config.colors.center_point;
                    vert.color.a -= 50;
                } else {
                    vert.color = bone.group_color;
                    vert.color.a -= 25;
                }
            }

            // add this rect to drawing buffers
            verts.append(&mut world_verts);
            let mut bone_indices = bone.indices.clone();
            add_offseted_indices(&mut bone_indices, &mut indices);
        }

        // draw rects
        setup_render_buffer(&mut renderer.rect_buffer, &verts, &indices, queue);
        draw(&renderer.rect_buffer, render_pass, 0, indices.len());
    }

    // render mesh wireframes if on
    if renderer.render_mesh_wf {
        let mut verts = vec![];
        let mut indices = vec![];
        render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);
        for bone in &temp_arm.bones {
            let already_editing =
                edit_mode.showing_mesh && armature.sel_bone(&sel).unwrap().id == bone.id;
            let is_hidden = temp_arm.is_bone_hidden(false, config.propagate_visibility, bone.id);
            if !bone.verts_edited || is_hidden || already_editing {
                continue;
            }

            let mouse = mouse_world_vert;
            let nw = &mut new_vert;
            let wv = bone.world_verts.clone();

            #[rustfmt::skip]
            let (mut verts_p, mut indices_p, _) = bone_vertices(&wv, false, selections, input, camera, config, events, armature, renderer, -1);
            #[rustfmt::skip]
            let (mut verts_l, mut indices_l, _) = vert_lines(bone, &temp_arm.bones, &mouse, nw, false, false, camera, input, selections, events);

            // color wireframes
            if bone.group_color.a == 0 {
                for vert in &mut verts_p {
                    vert.color = config.colors.center_point;
                    vert.color.a = vert.color.a.saturating_sub(75);
                }
                for vert in &mut verts_l {
                    vert.color = config.colors.center_point;
                    vert.color.a = vert.color.a.saturating_sub(125);
                }
            } else {
                for vert in &mut verts_p {
                    vert.color = bone.group_color;
                    vert.color.a = vert.color.a.saturating_sub(75);
                }
                for vert in &mut verts_l {
                    vert.color = bone.group_color;
                    vert.color.a = vert.color.a.saturating_sub(125);
                }
            }

            verts.append(&mut verts_p);
            verts.append(&mut verts_l);
            add_offseted_indices(&mut indices_p, &mut indices);
            add_offseted_indices(&mut indices_l, &mut indices);
        }
        setup_render_buffer(&mut renderer.meshframe_buffer, &verts, &indices, queue);
        draw(&renderer.meshframe_buffer, render_pass, 0, indices.len());
    }

    if new_vert != None {
        renderer.new_vert = new_vert;
        events.select_vertex(-1, false);
        events.new_vertex();
    }

    if config.gridline_front {
        draw_gridline(render_pass, renderer, &camera, &config, queue);
    }

    #[rustfmt::skip]
    draw_points_and_kites(config, camera, input, edit_mode, &mut temp_arm, selected_bone_ids, selections, renderer, queue, render_pass, events, armature);

    // check if this bone is part of IK, to disable editing later
    let mut has_ik = false;
    if let Some(bone) = armature.sel_bone(&sel) {
        has_ik = bone.ik_family_id != -1
            && !bone.ik_disabled
            && armature.bone_eff(bone.id) != JointEffector::Start;
    }

    // show transform rings when editing a bone
    let ring_enabled = config.transform_rot_radius > 0. && config.transform_scale_radius > 0.;
    let idle_mouse = !input.left_down && !input.left_clicked || camera.on_ui;
    let selected = armature.sel_bone(&sel) != None && selections.bone_ids.len() == 1;
    if !edit_mode.showing_mesh && !has_ik && idle_mouse && selected && ring_enabled {
        #[rustfmt::skip]
        transform_ring(config, camera, armature, &mut temp_arm, render_pass, renderer, events, edit_mode, &sel, queue, &mouse_pos);
    }

    // if no SelectBone events have been called, unselect current if mouse is pressed
    if !camera.on_ui
        && armature.bones.len() > 0
        && edit_mode.sel_time > 0.25
        && input.left_clicked
        && !edit_mode.showing_mesh
    {
        let mut unselect = true;
        for event in &events.events {
            if *event == Events::SelectBone {
                unselect = false;
                break;
            }
        }
        if unselect {
            events.select_bone(usize::MAX, true);
        }
    }

    // if cursor is not hovering on any verts, unselect on click
    if !camera.on_ui
        && input.left_clicked
        && (edit_mode.showing_mesh || selections.bind != -1)
        && !hovered_vert
    {
        events.select_vertex(-1, false);
    }

    // dragging vert stuff
    if !input.left_down {
        renderer.editing_bone = false;
        renderer.started_dragging_verts = false;
    } else if sel.vert_ids.len() > 0 && armature.sel_bone(&sel) != None && !camera.on_ui {
        if !renderer.started_dragging_verts {
            events.save_bone(selections.bone_idx);
            renderer.started_dragging_verts = true
        }
        for vert_id in sel.vert_ids.clone() {
            events.drag_vertex(vert_id);
        }

        return;
    }

    // keep track of selected bone's initial rotation, if mouse isn't being pressed
    if input.mouse_init == None {
        if let Some(bone) = armature.sel_bone(&sel) {
            let anim_bones = &armature.animated_bones;
            let sel_anim_bone = anim_bones.iter().find(|b| b.id == bone.id).unwrap();
            renderer.bone_init_rot = sel_anim_bone.rot;
        }
    }

    if !input.left_down && !input.right_down {
        return;
    }

    // move camera
    if input.right_down && !camera.on_ui {
        let vel = renderer::mouse_vel(&input, &camera) * camera.zoom;
        events.edit_camera(camera.pos.x + vel.x, camera.pos.y + vel.y, camera.zoom);
        return;
    }

    // bone's transforms can't be edited if editing its verts, or it has Ik
    if edit_mode.showing_mesh || has_ik {
        return;
    }

    // editing bone transforms (move, rotate, scale)
    let idx = sel.bone_idx;
    let input = &input;
    let mouse_moved = input.mouse != input.mouse_prev_left;
    if camera.on_ui {
        renderer.editing_bone = false;
    } else if idx != usize::MAX && input.left_down && hover_bone_id == -1 {
        // only register edits if mouse is moving
        if mouse_moved {
            events.update_current_editing(0);
        }

        // prioritize temporary edits (from transform rings) over edit mode
        let current_edit = if edit_mode.temporary == None {
            &edit_mode.current
        } else {
            edit_mode.temporary.as_ref().unwrap()
        };

        // save bone to undo stack
        if !renderer.editing_bone {
            events.save_edited_bone(selections.bone_idx);
            renderer.editing_bone = true;
        }

        let mut line_verts = vec![];
        let mut line_indices = vec![];

        // move all selected (root) bones
        for sel_id in &selections.only_root_bones(&armature.bones) {
            if *current_edit == EditModes::Rotate {
                let mut mouse = utils::screen_to_world_space(input.mouse, camera.window);
                mouse.x *= camera.aspect_ratio();
                let bone = temp_arm.bones.iter().find(|b| b.id == *sel_id).unwrap();
                let center = vert(Some(bone.pos), None, None);
                let cam = &world_camera(&camera, &config);
                let aspect_ratio = camera.aspect_ratio();
                let cw = world_vert(center, cam, aspect_ratio, Vec2::new(0.5, 0.5));
                let (mut verts, mut indices) = draw_line(cw.pos, mouse);
                line_verts.append(&mut verts);
                add_offseted_indices(&mut indices, &mut line_indices);
            }
            if !mouse_moved {
                continue;
            }
            let bone = temp_arm.bones.iter().find(|b| b.id == *sel_id).unwrap();
            #[rustfmt::skip]
            edit_bone(events, edit_mode, current_edit.clone(), &selections, &camera, &config, &input, &renderer, bone, &temp_arm.bones, &mouse_pos);
        }

        if *current_edit == EditModes::Rotate {
            render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);
            setup_render_buffer(&mut renderer.ring_buffer, &line_verts, &line_indices, queue);
            let buffer = &mut renderer.ring_buffer;
            draw(buffer, render_pass, 0, line_indices.len());
        }
    }
}

pub fn draw_armature(
    armature: &Armature,
    src_arm: &Armature,
    showing_mesh: bool,
    sel: &SelectionState,
    config: &Config,
    queue: &wgpu::Queue,
    render_pass: &mut RenderPass,
    buffer: &RenderBuffer,
) {
    let mut all_verts = vec![];
    let mut all_indices = vec![];

    // keep track of which bones should be rendered
    let mut bone_ids_to_draw = vec![];

    for b in 0..armature.bones.len() {
        let tex = src_arm.tex_of(armature.bones[b].id);
        let id = armature.bones[b].id;
        let bone = &armature.bones[b];
        let hidden = armature.is_bone_hidden(false, config.propagate_visibility, id);
        if bone.world_verts.len() == 0 || tex == None || hidden {
            continue;
        }
        if showing_mesh
            && (src_arm.sel_bone(&sel) != None && src_arm.sel_bone(&sel).unwrap().id == id)
        {
            continue;
        }
        let mut world_verts = bone.world_verts.clone();
        for vert in &mut world_verts {
            vert.color = Color::new(255, 255, 255, 255);
        }
        bone_ids_to_draw.push(armature.bones[b].id);
        all_verts.append(&mut world_verts);
        add_offseted_indices(&mut bone.indices.clone(), &mut all_indices);
    }
    setup_render_buffer(buffer, &all_verts, &all_indices, queue);

    let mut curr_indices = 0;
    for bone_id in bone_ids_to_draw {
        let tex = src_arm.tex_of(bone_id);
        let bone = &armature.bones.iter().find(|b| b.id == bone_id).unwrap();
        let t = tex.unwrap();
        let bg = &src_arm.tex_data(t).unwrap().bind_group;
        let indices_end = curr_indices + bone.indices.len();
        render_pass.set_bind_group(0, bg, &[]);
        draw(&buffer, render_pass, curr_indices, indices_end);
        curr_indices += bone.indices.len();
    }
}

pub fn world_camera(camera: &Camera, config: &Config) -> Camera {
    let mut cam = camera.clone();
    match config.layout {
        UiLayout::Right => cam.pos.x += 1500. * camera.aspect_ratio(),
        UiLayout::Left => cam.pos.x -= 1500. * camera.aspect_ratio(),
        _ => {}
    };
    cam
}

pub fn sel_tex_img(bone: &Bone, armature: &Armature) -> image::DynamicImage {
    let tex = armature.tex_of(bone.id).unwrap();
    armature.tex_data(tex).unwrap().image.clone()
}

pub fn mouse_vel(input: &InputStates, camera: &Camera) -> Vec2 {
    let mouse_world = utils::screen_to_world_space(input.mouse, camera.window);
    let mouse_prev_world = utils::screen_to_world_space(input.mouse_prev, camera.window);
    mouse_prev_world - mouse_world
}

fn vert(pos: Option<Vec2>, col: Option<Color>, uv: Option<Vec2>) -> Vertex {
    Vertex {
        pos: pos.unwrap_or_default(),
        color: col.unwrap_or_default(),
        uv: uv.unwrap_or_default(),
        ..Default::default()
    }
}

fn tri_point(p: &Vec2, a: &Vec2, b: &Vec2, c: &Vec2) -> (f32, f32, f32, f32) {
    let s = a.y * c.x - a.x * c.y + (c.y - a.y) * p.x + (a.x - c.x) * p.y;
    let t = a.x * b.y - a.y * b.x + (a.y - b.y) * p.x + (b.x - a.x) * p.y;

    if (s < 0.0) != (t < 0.0) && s != 0.0 && t != 0.0 {
        return (-1., -1., -1., -1.);
    }

    let area = -b.y * c.x + a.y * (c.x - b.x) + a.x * (b.y - c.y) + b.x * c.y;
    if area == 0.0 {
        return (-1., -1., -1., -1.);
    }

    let s_normalized = s / area;
    let t_normalized = t / area;

    if s_normalized >= 0.0 && t_normalized >= 0.0 && (s_normalized + t_normalized) <= 1.0 {
        let third = 1. - (s_normalized + t_normalized);
        return (area, s_normalized, t_normalized, third);
    }

    (-1., -1., -1., -1.)
}

pub fn get_sprite_boundary(armature: &Armature, camera: &Camera, config: &Config) -> (Vec2, Vec2) {
    let mut temp_arm = Armature::default();
    temp_arm.bones = armature.bones.clone();
    construction(&mut temp_arm.bones, &armature.bones);
    temp_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    let mut cam = world_camera(&camera, &config).clone();
    cam.pos = Vec2::new(0., 0.);
    cam.zoom = 1.;

    let mut left_top = Vec2::new(f32::MAX, -f32::MAX);
    let mut right_bot = Vec2::new(-f32::MAX, f32::MAX);

    for b in 0..temp_arm.bones.len() {
        if armature.tex_of(temp_arm.bones[b].id) == None || temp_arm.bones[b].hidden {
            continue;
        }

        for v in 0..temp_arm.bones[b].vertices.len() {
            let tb = &temp_arm.bones[b];
            let new_vert = world_vert(tb.vertices[v], &cam, 1., Vec2::default());

            let pos = new_vert.pos;
            left_top = Vec2::new(left_top.x.min(pos.x), left_top.y.max(pos.y));
            right_bot = Vec2::new(right_bot.x.max(pos.x), right_bot.y.min(pos.y));
        }
    }

    (left_top, right_bot)
}

/// Stripped-down renderer for screenshot purposes.
pub fn render_screenshot(
    render_pass: &mut RenderPass,
    armature: &Armature,
    camera: &Camera,
    config: &Config,
    renderer: &Renderer,
    queue: &wgpu::Queue,
) {
    let mut temp_arm = Armature::default();
    temp_arm.bones = armature.bones.clone();
    construction(&mut temp_arm.bones, &armature.bones);
    temp_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));
    let sel = SelectionState::default();

    for b in 0..temp_arm.bones.len() {
        if armature.tex_of(temp_arm.bones[b].id) == None
            || temp_arm.is_bone_hidden(false, config.propagate_visibility, temp_arm.bones[b].id)
        {
            continue;
        }

        for v in 0..temp_arm.bones[b].vertices.len() {
            let tb = &temp_arm.bones[b];
            let mut new_vert = world_vert(tb.vertices[v], camera, 1., Vec2::default());
            new_vert.tint = temp_arm.bones[b].tint;
            temp_arm.bones[b].world_verts.push(new_vert);
        }
    }
    #[rustfmt::skip]
    draw_armature(&temp_arm, armature, false, &sel, config, queue, render_pass, &renderer.bone_buffer);
}

pub fn construction(bones: &mut Vec<Bone>, og_bones: &Vec<Bone>) {
    inheritance(bones, std::collections::HashMap::new(), &vec![]);

    let mut ik_rot: std::collections::HashMap<i32, f32> = std::collections::HashMap::new();

    let mut done_ids: Vec<i32> = vec![];
    for b in 0..bones.len() {
        let ik_id = bones[b].ik_family_id;
        if bones[b].ik_disabled || ik_id == -1 || done_ids.contains(&ik_id) {
            continue;
        }

        done_ids.push(bones[b].ik_family_id);

        let target = bones.iter().find(|bone| bone.id == bones[b].ik_target_id);
        if target == None {
            continue;
        }

        let family: Vec<&Bone> = bones.iter().filter(|b| b.ik_family_id == ik_id).collect();
        let mut joints = vec![];
        for bone in family {
            joints.push(bone.clone());
        }

        inverse_kinematics(&mut joints, target.unwrap().pos);

        // save rotations for the next forward kinematics call
        for j in 0..joints.len() {
            if j == joints.len() - 1 {
                continue;
            }
            ik_rot.insert(joints[j].id, joints[j].rot);
        }
    }

    // re-construct bones, accounting for rotations saved from IK
    *bones = og_bones.clone();
    inheritance(bones, ik_rot.clone(), &vec![]);

    construct_verts(bones);
}

pub fn runtime_construction(
    bones: &mut Vec<Bone>,
    og_bones: &Vec<Bone>,
    armature_bones: &mut Vec<Bone>,
) {
    inheritance(bones, std::collections::HashMap::new(), &vec![]);

    let mut ik_rot: std::collections::HashMap<i32, f32> = std::collections::HashMap::new();

    let mut done_ids: Vec<i32> = vec![];
    for b in 0..bones.len() {
        let ik_id = bones[b].ik_family_id;
        if bones[b].ik_disabled || ik_id == -1 || done_ids.contains(&ik_id) {
            continue;
        }

        done_ids.push(bones[b].ik_family_id);

        let target = bones.iter().find(|bone| bone.id == bones[b].ik_target_id);
        if target == None {
            continue;
        }

        let family: Vec<&Bone> = bones.iter().filter(|b| b.ik_family_id == ik_id).collect();
        let mut joints = vec![];
        for bone in family {
            joints.push(bone.clone());
        }

        inverse_kinematics(&mut joints, target.unwrap().pos);

        // save rotations for the next forward kinematics call
        for j in 0..joints.len() {
            if j == joints.len() - 1 {
                continue;
            }
            ik_rot.insert(joints[j].id, joints[j].rot);
        }
    }

    // re-construct bones, accounting for rotations saved from IK
    *bones = og_bones.clone();
    inheritance(bones, ik_rot.clone(), &vec![]);

    simulate_physics(armature_bones, bones);

    // re-construct bones, accounting for physics
    *bones = og_bones.clone();
    inheritance(bones, ik_rot.clone(), &armature_bones);

    construct_verts(bones);
}

// simulate physics on the armature, then apply it to constructed bones
fn simulate_physics(armature_bones: &mut Vec<Bone>, constructed_bones: &mut Vec<Bone>) {
    for b in 0..armature_bones.len() {
        let s = Vec2::new(0.3, 0.3);
        let e = Vec2::new(0.6, 0.6);
        let arm_bone = &mut armature_bones[b];
        let const_bone = &constructed_bones[b];
        let prev_pos = arm_bone.phys_global_pos;

        // interpolate position
        if arm_bone.phys_pos_damping > 0. || arm_bone.phys_rot_resistance > 0. {
            let phys_pos = &mut arm_bone.phys_global_pos;
            let damping = arm_bone.phys_pos_damping;
            phys_pos.x = utils::interp(2, damping as i32, phys_pos.x, const_bone.pos.x, s, e);
            phys_pos.y = utils::interp(2, damping as i32, phys_pos.y, const_bone.pos.y, s, e);
        }

        // interpolate scale
        if arm_bone.phys_scale_damping > 0. {
            let phys_scale = &mut arm_bone.phys_global_scale;
            let elas = arm_bone.phys_scale_damping;
            phys_scale.x = utils::interp(2, elas as i32, phys_scale.x, const_bone.scale.x, s, e);
            phys_scale.y = utils::interp(2, elas as i32, phys_scale.y, const_bone.scale.y, s, e);
        }

        // interpolate rotation
        if arm_bone.phys_rot_damping > 0. {
            let rot = utils::shortest_angle_delta(arm_bone.phys_global_rot, const_bone.rot);
            arm_bone.phys_global_rot += rot / arm_bone.phys_rot_damping;
        }

        // interpolate parent orbit (rot res, bounce, etc)
        let bones = &constructed_bones;
        let parent = bones.iter().find(|b| b.id == const_bone.parent_id);
        if arm_bone.phys_rot_resistance > 0. && parent != None {
            // interpolate to the angle difference between bone and parent
            let diff = (const_bone.pos - parent.unwrap().pos).normalize();
            let diff_angle = diff.y.atan2(diff.x);
            let mut rest_rot = shortest_angle_delta(arm_bone.phys_global_orbit, diff_angle);
            // apply bounce
            if arm_bone.phys_rot_bounce > 0. && arm_bone.phys_rot_bounce <= 1. {
                rest_rot += arm_bone.phys_global_orbit_vel / (2. - arm_bone.phys_rot_bounce);
                arm_bone.phys_global_orbit_vel = rest_rot;
            }
            arm_bone.phys_global_orbit += rest_rot / 10.;

            // swing orbit based on position momentum
            let vel = (arm_bone.phys_global_pos - prev_pos).normalize();
            let angle = (-vel.y).atan2(-vel.x);
            let vel_rot = utils::shortest_angle_delta(arm_bone.phys_global_orbit, angle);
            let strength = (arm_bone.phys_global_pos - prev_pos).mag();
            arm_bone.phys_global_orbit += vel_rot * strength / arm_bone.phys_rot_resistance;

            // apply difference in final angle and orbit
            arm_bone.phys_global_orbit_diff = diff_angle - arm_bone.phys_global_orbit;
        }
    }
}

pub fn construct_verts(bones: &mut Vec<Bone>) {
    for b in 0..bones.len() {
        let bone = bones[b].clone();

        // track vertex init pos for binds
        let mut init_vert_pos = vec![];
        for vert in &mut bones[b].vertices {
            init_vert_pos.push(vert.pos);
        }

        // move vertex to main bone.
        // this will be overridden if vertex has a bind.
        for vert in &mut bones[b].vertices {
            vert.pos = inherit_vert(vert.pos, &bone);
            vert.offset_rot = 0.;
        }

        for bi in 0..bones[b].binds.len() {
            let b_id = bones[b].binds[bi].bone_id;
            if b_id == -1 {
                continue;
            }

            // delete bind if its bind bone doesn't exist
            let bind_bone_raw = bones.iter().find(|bone| bone.id == b_id);
            if bind_bone_raw == None {
                bones[b].binds.remove(bi);
                break;
            }

            let bind_bone = bind_bone_raw.unwrap().clone();
            let bind = bones[b].binds[bi].clone();
            for v_id in 0..bind.verts.len() {
                let id = bind.verts[v_id].id as u32;
                let idx = bones[b].vertices.iter().position(|vert| vert.id == id);

                if !bind.is_path {
                    // weights
                    let vert = &mut bones[b].vertices[idx.unwrap()];
                    let weight = bind.verts[v_id].weight;
                    let end_pos = inherit_vert(init_vert_pos[idx.unwrap()], &bind_bone) - vert.pos;
                    vert.pos += end_pos * weight;
                    continue;
                }

                // pathing:
                // Bone binds are treated as one continuous line.
                // Vertices will follow along this path.

                // get previous and next bone
                let normal_angle = get_path_normal_angle(bones, &bone, bi);
                if normal_angle == f32::MAX {
                    continue;
                }

                // move vertex to bind bone, then just adjust it to 'bounce' off the line's surface
                let vert = &mut bones[b].vertices[idx.unwrap()];
                vert.pos = init_vert_pos[idx.unwrap()] + bind_bone.pos;
                let rotated = utils::rotate(&(vert.pos - bind_bone.pos), normal_angle);
                vert.pos = bind_bone.pos + (rotated * bind.verts[v_id].weight);
                vert.offset_rot = normal_angle;
            }
        }
    }
}

pub fn get_path_normal_angle(bones: &Vec<Bone>, bone: &Bone, bind_idx: usize) -> f32 {
    let prev = if bind_idx > 0 { bind_idx - 1 } else { bind_idx };
    let next = (bind_idx + 1).min(bone.binds.len() - 1);
    if bone.binds[prev].bone_id == -1 || bone.binds[next].bone_id == -1 {
        return f32::MAX;
    }
    let bind_bone = bones.iter().find(|b| b.id == bone.binds[bind_idx].bone_id);
    let prev_bone = bones.iter().find(|b| b.id == bone.binds[prev].bone_id);
    let next_bone = bones.iter().find(|b| b.id == bone.binds[next].bone_id);

    // get the average of normals between previous bone, this bone, and next bone
    let prev_dir = bind_bone.unwrap().pos - prev_bone.unwrap().pos;
    let next_dir = next_bone.unwrap().pos - bind_bone.unwrap().pos;
    let prev_normal = Vec2::new(-prev_dir.y, prev_dir.x).normalize();
    let next_normal = Vec2::new(-next_dir.y, next_dir.x).normalize();
    let average = prev_normal + next_normal;
    let normal_angle = average.y.atan2(average.x);

    normal_angle
}

pub fn inherit_vert(mut pos: Vec2, bone: &Bone) -> Vec2 {
    pos *= bone.scale;
    pos = utils::rotate(&pos, bone.rot);
    pos += bone.pos;
    pos
}

pub fn inverse_kinematics(bones: &mut Vec<Bone>, target: Vec2) {
    let root = bones[0].pos;

    if bones[0].ik_mode == InverseKinematicsMode::FABRIK {
        for _ in 0..10 {
            fabrik(bones, root, target);
        }
    } else {
        arc_ik(bones, root, target)
    }

    // rotating bones
    let end_bone = bones.last().unwrap();
    let mut tip_pos = end_bone.pos;
    for b in (0..bones.len()).rev() {
        let dir = tip_pos - bones[b].pos;
        bones[b].rot = dir.y.atan2(dir.x);
        tip_pos = bones[b].pos;
    }

    // apply constraints if this IK has more than 1 bone
    if bones.len() == 1 {
        return;
    }
    let joint_dir = (bones[1].pos - bones[0].pos).normalize();
    let base_dir = (target - root).normalize();
    let dir = joint_dir.x * base_dir.y - base_dir.x * joint_dir.y;
    let base_angle = base_dir.y.atan2(base_dir.x);

    let cw = bones[0].ik_constraint == JointConstraint::Clockwise && dir > 0.;
    let ccw = bones[0].ik_constraint == JointConstraint::CounterClockwise && dir < 0.;
    if ccw || cw {
        for b in 0..bones.len() {
            bones[b].rot = -bones[b].rot + base_angle * 2.;
        }
    }
}

pub fn arc_ik(bones: &mut Vec<Bone>, root: Vec2, target: Vec2) {
    // determine where bones will be on the arc line (ranging from 0 to 1)
    let mut dist: Vec<f32> = vec![0.];

    let max_length = (bones.last().unwrap().pos - root).mag();
    let mut curr_length = 0.;
    for b in 1..bones.len() {
        let length = (bones[b].pos - bones[b - 1].pos).mag();
        curr_length += length;
        dist.push(curr_length / max_length);
    }

    let base = target - root;
    let base_angle = base.y.atan2(base.x);
    let base_mag = base.mag().min(max_length);
    let peak = max_length / base_mag;
    let valley = base_mag / max_length;

    for b in 1..bones.len() {
        bones[b].pos = Vec2::new(
            bones[b].pos.x * valley,
            root.y + (1. - peak) * (dist[b] * 3.14).sin() * base_mag,
        );

        let rotated = utils::rotate(&(bones[b].pos - root), base_angle);
        bones[b].pos = rotated + root;
    }
}

// https://www.youtube.com/watch?v=NfuO66wsuRg
pub fn fabrik(bones: &mut Vec<Bone>, root: Vec2, target: Vec2) {
    // forward-reaching
    let mut next_pos: Vec2 = target;
    let mut next_length = 0.;
    for b in (0..bones.len()).rev() {
        let length = (next_pos - bones[b].pos).normalize() * next_length;
        if b != 0 {
            next_length = (bones[b].pos - bones[b - 1].pos).mag();
        }
        bones[b].pos = next_pos - length;
        next_pos = bones[b].pos;
    }

    // backward-reaching
    let mut prev_pos: Vec2 = root;
    let mut prev_length = 0.;
    for b in 0..bones.len() {
        let length = (prev_pos - bones[b].pos).normalize() * prev_length;
        if b != bones.len() - 1 {
            prev_length = (bones[b].pos - bones[b + 1].pos).mag();
        }
        bones[b].pos = prev_pos - length;
        prev_pos = bones[b].pos;
    }
}

pub fn edit_bone(
    events: &mut EventState,
    edit_mode: &EditMode,
    current_edit: EditModes,
    selections: &SelectionState,
    camera: &Camera,
    config: &Config,
    input: &InputStates,
    renderer: &Renderer,
    bone: &Bone,
    bones: &Vec<Bone>,
    mouse_pos: &Vec2,
) {
    let mut anim_id = selections.anim;
    let anim_frame = selections.anim_frame;
    if !edit_mode.anim_open {
        anim_id = usize::MAX;
    }

    macro_rules! edit {
        ($bone:expr, $element:expr, $value:expr) => {
            events.edit_bone($bone.id, &$element, $value, "", anim_id, anim_frame);
        };
    }

    let vert = vert(Some(bone.pos), None, None);
    let cam = &world_camera(&camera, &config);
    let bone_center = world_vert(vert, cam, camera.aspect_ratio(), Vec2::new(0.5, 0.5));

    // mouse velocity to be used for moving and scaling
    let mut mouse_vel = mouse_vel(&input, &camera) * camera.zoom;

    // snap mouse velocity to X or Y, depending on which is faster
    let strictness = 3.; // when an axis is triggered, how much should the other be to overrride it?
    let deadzone = 5.; // axis must be faster than this to be triggered
    if edit_mode.holding_edit_snap {
        let norm = Vec2::new(mouse_vel.normalize().x.abs(), mouse_vel.normalize().y.abs());
        if mouse_vel.x.abs() > deadzone && norm.x > norm.y * strictness {
            mouse_vel.y = 0.;
        } else if mouse_vel.y.abs() > deadzone && norm.y > norm.x * strictness {
            mouse_vel.x = 0.;
        } else {
            mouse_vel = Vec2::new(0., 0.);
        }
    }

    if current_edit == EditModes::Move {
        let mut pos = bone.pos;
        pos -= mouse_vel;

        // restore universal position by offsetting against parents' attributes
        if bone.parent_id != -1 {
            let parent = bones.iter().find(|b| b.id == bone.parent_id).unwrap();
            pos -= parent.pos;
            pos = utils::rotate(&pos, -parent.rot);
            pos /= parent.scale;
            if pos.x.is_nan() {
                pos.x = 0.;
            }
            if pos.y.is_nan() {
                pos.y = 0.;
            }
        }

        if pos.x != bone.pos.x {
            edit!(bone, AnimElement::PositionX, pos.x);
        }
        if pos.y != bone.pos.y {
            edit!(bone, AnimElement::PositionY, pos.y);
        }
    } else if current_edit == EditModes::Rotate {
        let mouse_init = utils::screen_to_world_space(input.mouse_init.unwrap(), camera.window);
        let dir_init = mouse_init - bone_center.pos;
        let rot_init = dir_init.y.atan2(dir_init.x);

        let mouse = utils::screen_to_world_space(input.mouse, camera.window);
        let dir = mouse - bone_center.pos;
        let rot = dir.y.atan2(dir.x);

        let mut rot = renderer.bone_init_rot + (rot - rot_init);

        // snap rot to user-defined steps if holding snap key
        let step = config.rot_snap_step * 3.14 / 180.;
        if edit_mode.holding_edit_snap {
            rot = (rot / step).round() * step
        }

        if rot != bone.rot {
            edit!(bone, AnimElement::Rotation, rot);
        }
    } else if current_edit == EditModes::Scale {
        let mut scale = bone.scale;

        // restore universal scale, by offsetting against parent's
        if bone.parent_id != -1 {
            let parent = bones.iter().find(|b| b.id == bone.parent_id).unwrap();
            scale /= parent.scale;
        }

        scale -= mouse_vel / camera.zoom;

        // maintain aspect ratio (same X and Y scale) if holding edit mod
        if edit_mode.holding_edit_mod {
            let distance = (*mouse_pos - bone.pos).mag() / camera.zoom * 3.;
            scale = Vec2::new(distance, distance);
        }

        if scale.x != bone.scale.x {
            edit!(bone, AnimElement::ScaleX, scale.x);
        }
        if scale.y != bone.scale.y {
            edit!(bone, AnimElement::ScaleY, scale.y);
        }
    }
}

pub fn inheritance(
    bones: &mut Vec<Bone>,
    ik_rot: std::collections::HashMap<i32, f32>,
    arm_bones: &Vec<Bone>,
) {
    for i in 0..bones.len() {
        let mut parent: Option<Bone> = None;
        for b in 0..bones.len() {
            if bones[b].id == bones[i].parent_id {
                parent = Some(bones[b].clone());
                break;
            }
        }

        // inherit parent
        if let Some(parent) = parent.clone() {
            bones[i].rot += parent.rot;
            bones[i].scale *= parent.scale;

            // adjust bone's position based on parent's scale
            bones[i].pos *= parent.scale;

            // orbit the parent
            let mut orbit_rot = parent.rot;
            // apply orbital difference, if rotation resistance physics is active
            if arm_bones.len() > 0 && bones[i].phys_rot_resistance > 0. {
                orbit_rot -= bones[i].phys_global_orbit_diff
            }
            bones[i].pos = utils::rotate(&bones[i].pos, orbit_rot);

            // inherit position from parent
            bones[i].pos += parent.pos;
        }

        // apply rotations from IK, if provided
        let ik_rot = ik_rot.get(&bones[i].id);
        if ik_rot != None {
            bones[i].rot = *ik_rot.unwrap();
        }

        // apply physics, if armature_bones is provided
        if arm_bones.len() > 0 {
            if bones[i].phys_rot_damping > 0. {
                bones[i].rot = arm_bones[i].phys_global_rot;
            }
            if bones[i].phys_pos_damping > 0. {
                bones[i].pos = arm_bones[i].phys_global_pos;
            }
            if bones[i].phys_scale_damping > 0. {
                bones[i].scale = arm_bones[i].phys_global_scale;
            }
        }
    }
}

pub fn draw(
    buffer: &RenderBuffer,
    render_pass: &mut RenderPass,
    indices_start: usize,
    indices_end: usize,
) {
    render_pass.set_vertex_buffer(0, buffer.vertex.as_ref().unwrap().slice(..));
    render_pass.set_index_buffer(
        buffer.index.as_ref().unwrap().slice(..),
        wgpu::IndexFormat::Uint32,
    );
    render_pass.draw_indexed(indices_start as u32..indices_end as u32, 0, 0..1);
}

pub fn bone_vertices(
    world_verts: &Vec<Vertex>,
    editable: bool,
    selections: &SelectionState,
    input: &InputStates,
    camera: &Camera,
    config: &Config,
    events: &mut EventState,
    armature: &Armature,
    renderer: &mut Renderer,
    hover_id: i32,
) -> (Vec<Vertex>, Vec<u32>, i32) {
    let mut all_verts = vec![];
    let mut all_indices = vec![];
    let mut hovering_vert_id = -1;
    let v2z = Vec2::ZERO;
    let rotated = 45. * 3.14 / 180.;
    let sel = selections.clone();
    let radius = config.center_point_radius;

    #[rustfmt::skip]
    macro_rules! point {
        ($idx:expr, $color:expr, $size:expr) => {
            draw_point(&world_verts[$idx].pos, &camera, &config, &v2z, $color, v2z, rotated, radius * camera.zoom * $size)
        };
    }
    macro_rules! add_point {
        ($verts:expr, $indices:expr, $wv:expr) => {
            for i in &mut $indices {
                *i += $wv as u32 * 4;
            }
            all_verts.append(&mut $verts);
            all_indices.append(&mut $indices);
        };
    }

    for wv in 0..world_verts.len() {
        let size = if world_verts[wv].id == hover_id as u32 {
            1.5
        } else {
            1.
        };
        let idx = selections.bind;
        let verts: Vec<i32>;
        if idx == -1 {
            verts = vec![];
        } else {
            let selected = armature.sel_bone(&sel).unwrap();
            let sel_bind = &selected.binds;
            verts = sel_bind[idx as usize].verts.iter().map(|v| v.id).collect();
        }

        // yellow vertex if bound
        let bound = idx != -1 && verts.contains(&(world_verts[wv].id as i32));
        let white = Color::new(255, 255, 255, 255);
        let mut col = if bound {
            Color::new(255, 255, 0, 255)
        } else {
            Color::new(0, 255, 0, 255)
        };

        // white vertex if selected
        let selected = selections.vert_ids.contains(&(world_verts[wv].id as usize));
        if selected {
            col = white;
        } else {
            col.a = if editable { 125 } else { 38 };
        }

        let (mut verts, mut indices) = point!(wv, col, size);
        let mouse_on_it = utils::in_bounding_box(&input.mouse, &verts, &camera.window).1;
        if mouse_on_it {
            hovering_vert_id = world_verts[wv].id as i32;
        }

        if camera.on_ui || !mouse_on_it || !editable {
            add_point!(verts, indices, wv);
            continue;
        }

        let (mut verts, mut indices) = point!(wv, col, size);
        add_point!(verts, indices, wv);
        if input.right_clicked {
            if world_verts.len() <= 4 {
                events.open_modal("vert_limit", false);
            } else {
                events.remove_vertex(wv);
                break;
            }
        }

        if input.left_pressed {
            events.select_vertex(world_verts[wv].id as i32, false);
        }

        if input.left_clicked {
            // simulate double-click for binding verts
            if renderer.clicked_vert_id != world_verts[wv].id as i32 {
                renderer.clicked_vert_id = world_verts[wv].id as i32;
            } else if selections.bind != -1 {
                renderer.clicked_vert_id = -1;
                events.click_vertex(wv);
                events.select_vertex(-1, false);
            }
            break;
        }
    }

    (all_verts, all_indices, hovering_vert_id)
}

fn bone_triangle(tb: &Bone, mouse_world_vert: &Vertex, wv: Vec<Vertex>) -> (u32, Vec<Vertex>) {
    let mut hovering_tri = vec![];
    let mut idx: usize = 0;
    for (i, chunk) in tb.indices.chunks_exact(3).enumerate() {
        let c0 = chunk[0] as usize;
        let c1 = chunk[1] as usize;
        let c2 = chunk[2] as usize;

        let bary = tri_point(&mouse_world_vert.pos, &wv[c0].pos, &wv[c1].pos, &wv[c2].pos);
        if bary.0 == -1. {
            continue;
        }

        hovering_tri.push(tb.world_verts[tb.indices[i * 3 + 0] as usize]);
        hovering_tri.push(tb.world_verts[tb.indices[i * 3 + 1] as usize]);
        hovering_tri.push(tb.world_verts[tb.indices[i * 3 + 2] as usize]);
        idx = i;
    }
    (idx as u32, hovering_tri)
}

pub fn vert_lines(
    bone: &Bone,
    bones: &Vec<Bone>,
    mouse_world_vert: &Vertex,
    new_vert: &mut Option<Vertex>,
    editable: bool,
    hovering_vert: bool,
    camera: &Camera,
    input: &InputStates,
    sel: &SelectionState,
    events: &mut EventState,
) -> (Vec<Vertex>, Vec<u32>, bool) {
    let mut added_vert = false;

    let mut all_verts: Vec<Vertex> = vec![];
    let mut all_indices: Vec<u32> = vec![];

    let mut indices = vec![0, 1, 2, 1, 2, 3];

    let mut hovered_once = false;

    // identify how many lines to create based on indices
    let mut lines: Vec<(u32, u32)> = vec![];
    for (_, chunk) in bone.indices.chunks_exact(3).enumerate() {
        // don't add duplicate lines
        macro_rules! doesnt_contain {
            ($first:expr, $second:expr) => {
                !lines.contains(&(chunk[$first], chunk[$second]))
                    && !lines.contains(&(chunk[$second], chunk[$first]))
            };
        }

        if doesnt_contain!(0, 1) {
            lines.push((chunk[0], chunk[1]));
        }
        if doesnt_contain!(1, 2) {
            lines.push((chunk[1], chunk[2]));
        }
        if doesnt_contain!(0, 2) {
            lines.push((chunk[0], chunk[2]));
        }
    }
    for l in 0..lines.len() {
        let i0 = lines[l].0;
        let i1 = lines[l].1;
        let v0 = bone.world_verts[i0 as usize];
        let v1 = bone.world_verts[i1 as usize];
        let dir = v0.pos - v1.pos;

        let width = 2. * (camera.zoom / 500.);
        let mut base = Vec2::new(width, width) / camera.zoom;
        base = utils::rotate(&base, dir.y.atan2(dir.x));

        let mut col = Color::new(0, 255, 0, 255);
        col -= Color::new(125, 125, 125, 0);
        col.a = if editable { 150 } else { 100 };

        #[rustfmt::skip]
        macro_rules! vert { ($pos:expr, $v:expr) => { Vertex { pos: $pos, color: col, ..$v } }; }

        let mut v0_top = vert!(v0.pos + base, v0);
        let mut v0_bot = vert!(v0.pos - base, v0);
        let mut v1_top = vert!(v1.pos + base, v1);
        let mut v1_bot = vert!(v1.pos - base, v1);

        let verts = vec![v0_top, v0_bot, v1_top, v1_bot];
        let add_color = Color::new(50, 50, 50, 255);

        let mut is_hovering = false;

        if editable && !hovering_vert && !camera.on_ui {
            for (_, chunk) in vec![0, 1, 2, 1, 2, 3].chunks_exact(3).enumerate() {
                let c0 = chunk[0] as usize;
                let c1 = chunk[1] as usize;
                let c2 = chunk[2] as usize;
                let mv = mouse_world_vert;
                let bary = tri_point(&mv.pos, &verts[c0].pos, &verts[c1].pos, &verts[c2].pos);
                if bary.0 == -1. {
                    continue;
                }
                is_hovering = true;

                let mouse_line = mouse_world_vert.pos - v0.pos;
                let whole_line = v1.pos - v0.pos;
                let interp = mouse_line.mag() / whole_line.mag();
                let uv = v0.uv + (v1.uv - v0.uv) * interp;

                if input.left_pressed {
                    let verts = &bone.vertices;
                    events.select_vertex(verts[i0 as usize].id as i32, false);
                    events.select_vertex(verts[i1 as usize].id as i32, true);
                } else if input.left_clicked && !added_vert {
                    let bones = &bones;
                    let v = &bones.iter().find(|b| b.id == bone.id).unwrap().vertices;
                    let wv0 = utils::rotate(&(v[i0 as usize].pos - bone.pos), -bone.rot);
                    let wv1 = utils::rotate(&(v[i1 as usize].pos - bone.pos), -bone.rot);
                    let pos = wv0 + (wv1 - wv0) * interp;
                    *new_vert = Some(vert(Some(pos / bone.scale), None, Some(uv)));
                    added_vert = true;
                }
            }

            if is_hovering {
                v0_top.add_color += add_color;
                v0_bot.add_color += add_color;
                v1_top.add_color += add_color;
                v1_bot.add_color += add_color;
            }

            let mv = &sel.vert_ids;

            if mv.len() == 2 && mv[0] == i0 as usize && mv[1] == i1 as usize {
                v0_top.add_color += add_color;
                v0_bot.add_color += add_color;
                v1_top.add_color += add_color;
                v1_bot.add_color += add_color;
            }
        }

        all_verts.append(&mut vec![v0_top, v0_bot, v1_top, v1_bot]);
        all_indices.append(&mut indices.clone());

        for i in &mut indices {
            *i += 4;
        }

        if is_hovering {
            hovered_once = true;
        }
    }

    (all_verts, all_indices, hovered_once)
}

fn draw_line(origin: Vec2, target: Vec2) -> (Vec<Vertex>, Vec<u32>) {
    let dir = target - origin;

    let width = 2.5;
    let mut base = Vec2::new(width, width) / 1000.;
    base = utils::rotate(&base, dir.y.atan2(dir.x) + (45. * 3.14 / 180.));

    let color = Color::new(0, 255, 0, 255);

    macro_rules! vert {
        ($pos:expr) => {
            vert(Some($pos), Some(color), None)
        };
    }

    let v0_top = vert!(origin - base);
    let v0_bot = vert!(origin + base);
    let v1_top = vert!(target - base);
    let v1_bot = vert!(target + base);

    let verts = vec![v0_top, v0_bot, v1_top, v1_bot];
    let indices = vec![0, 1, 2, 1, 2, 3];

    (verts, indices)
}

pub fn create_tex_rect(tex_size: &Vec2) -> (Vec<Vertex>, Vec<u32>) {
    #[rustfmt::skip]
    macro_rules! vert {
        ($pos:expr, $uv:expr, $id:expr) => { Vertex { pos: $pos, uv: $uv, id: $id, ..Default::default() } };
    }
    let tex = *tex_size / 2.;
    let mut verts = vec![
        vert!(Vec2::new(-tex.x, tex.y), Vec2::new(0., 0.), 0),
        vert!(Vec2::new(tex.x, tex.y), Vec2::new(1., 0.), 1),
        vert!(Vec2::new(tex.x, -tex.y), Vec2::new(1., 1.), 2),
        vert!(Vec2::new(-tex.x, -tex.y), Vec2::new(0., 1.), 3),
    ];
    verts = editor::sort_vertices(verts.clone());
    let indices = vec![0, 1, 2, 0, 2, 3];
    (verts, indices)
}

fn draw_point(
    offset: &Vec2,
    camera: &Camera,
    config: &Config,
    pos: &Vec2,
    color: Color,
    camera_pos: Vec2,
    rotation: f32,
    size: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let point_size = size;
    macro_rules! vert {
        ($pos:expr, $uv:expr) => {
            vert(Some($pos), Some(color), Some($uv))
        };
    }
    let verts: [Vertex; 4] = [
        vert!(Vec2::new(-point_size, point_size), Vec2::new(0., 1.)),
        vert!(Vec2::new(point_size, point_size), Vec2::new(1., 1.)),
        vert!(Vec2::new(-point_size, -point_size), Vec2::new(0., 0.)),
        vert!(Vec2::new(point_size, -point_size), Vec2::new(1., 0.)),
    ];

    draw_rect(verts, offset, camera, config, pos, camera_pos, rotation)
}

fn draw_flow_kite(
    offset: &Vec2,
    camera: &Camera,
    config: &Config,
    pos: &Vec2,
    color: Color,
    camera_pos: Vec2,
    rotation: f32,
    width: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let height = 20.;
    macro_rules! vert {
        ($pos:expr, $uv:expr) => {
            vert(Some($pos), Some(color), Some($uv))
        };
    }
    let verts: [Vertex; 4] = [
        vert!(Vec2::new(-1., height), Vec2::new(0., 1.)),
        vert!(Vec2::new(width, height), Vec2::new(0., 0.)),
        vert!(Vec2::new(-1., -height), Vec2::new(1., 1.)),
        vert!(Vec2::new(width, -height), Vec2::new(1., 0.)),
    ];

    draw_rect(verts, offset, camera, config, pos, camera_pos, rotation)
}

fn draw_rect(
    mut temp_verts: [Vertex; 4],
    offset: &Vec2,
    camera: &Camera,
    config: &Config,
    pos: &Vec2,
    camera_pos: Vec2, // set to (0, 0) for vertex points
    rotation: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    for v in &mut temp_verts {
        v.pos = utils::rotate(&v.pos, rotation);
        v.pos += *pos;
    }

    let mut point_verts = vec![];
    let ar = camera.aspect_ratio();
    let mut cam = world_camera(&camera, &config).clone();
    cam.pos = camera_pos;
    let pivot = Vec2::new(0., 0.);
    for vert in temp_verts {
        let vert = world_vert(vert, &cam, ar, pivot);
        point_verts.push(vert);
    }

    for vert in &mut point_verts {
        vert.pos += *offset;
    }

    (point_verts, vec![0, 1, 2, 1, 2, 3])
}

/// Get bind group of a texture.
pub fn create_texture_bind_group(
    pixels: Vec<u8>,
    dimensions: Vec2,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
) -> BindGroup {
    let tex_size = wgpu::Extent3d {
        width: dimensions.x as u32,
        height: dimensions.y as u32,
        depth_or_array_layers: 1,
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        size: tex_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: Some("diffuse_texture"),
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.x as u32),
            rows_per_image: Some(dimensions.y as u32),
        },
        tex_size,
    );

    let tex_view = tex.create_view(&wgpu::TextureViewDescriptor::default());

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        compare: None,
        ..Default::default()
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&tex_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: Some("diffuse_bind_group"),
    });

    bind_group
}

pub fn index_buffer(indices: Vec<u32>, device: &Device) -> wgpu::Buffer {
    wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        },
    )
}

pub fn vertex_buffer(vertices: &Vec<Vertex>, device: &Device) -> wgpu::Buffer {
    let gpu_verts: Vec<GpuVertex> = vertices.iter().map(|vert| (*vert).into()).collect();

    wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("index Buffer"),
            contents: bytemuck::cast_slice(&gpu_verts),
            usage: wgpu::BufferUsages::VERTEX,
        },
    )
}

pub fn world_vert(mut vert: Vertex, camera: &Camera, aspect_ratio: f32, pivot: Vec2) -> Vertex {
    vert.pos.x -= pivot.x;
    vert.pos.y += pivot.y;

    // offset bone with camera
    vert.pos -= camera.pos;

    // adjust for zoom level
    vert.pos /= camera.zoom;

    // adjust verts for aspect ratio
    vert.pos.x *= aspect_ratio;

    vert
}

fn draw_gridline(
    render_pass: &mut RenderPass,
    renderer: &mut Renderer,
    camera: &Camera,
    config: &Config,
    queue: &Queue,
) {
    render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);

    let cam = world_camera(camera, config);

    let col = Color::new(
        config.colors.gridline.r,
        config.colors.gridline.g,
        config.colors.gridline.b,
        255,
    );

    let width = 0.005 * cam.zoom;
    let regular_color = Color::new(col.r, col.g, col.b, 38);
    let highlight_color = Color::new(col.r, col.g, col.b, 255);

    let mut verts = vec![];
    let mut indices: Vec<u32> = vec![];
    let mut i: u32 = 0;

    // draw vertical lines
    let mut x = (cam.pos.x - cam.zoom / camera.aspect_ratio()).round();
    let right_side = cam.pos.x + cam.zoom / camera.aspect_ratio();
    while x < right_side {
        if x % config.gridline_gap as f32 != 0. {
            x += 1.;
            continue;
        }
        let color = if x == 0. {
            highlight_color
        } else {
            regular_color
        };
        verts.append(&mut draw_vertical_line(x, width, camera, config, color));
        indices.append(&mut vec![i, i + 1, i + 2]);
        i += 3;
        x += 1.;
    }

    // draw horizontal lines
    let mut y = (cam.pos.y - cam.zoom).round();
    let top_side = cam.pos.y + cam.zoom;
    while y < top_side {
        if y % config.gridline_gap as f32 != 0. {
            y += 1.;
            continue;
        }
        let color = if y == 0. {
            highlight_color
        } else {
            regular_color
        };
        verts.append(&mut draw_horizontal_line(y, width, camera, config, color));
        indices.append(&mut vec![i, i + 1, i + 2]);
        i += 3;
        y += 1.;
    }

    if verts.len() == 0 {
        return;
    }

    setup_render_buffer(&mut renderer.gridline_buffer, &verts, &indices, queue);
    draw(&renderer.gridline_buffer, render_pass, 0, indices.len());
}

macro_rules! vert {
    ($pos:expr, $color:expr) => {
        Vertex {
            pos: $pos,
            color: $color,
            ..Default::default()
        }
    };
}

pub fn draw_horizontal_line(
    y: f32,
    width: f32,
    camera: &Camera,
    config: &Config,
    color: Color,
) -> Vec<Vertex> {
    let edge = camera.zoom * 5.;
    let c = &world_camera(camera, config);
    let vertices: Vec<Vertex> = vec![
        vert!((Vec2::new(c.pos.x - edge, y) - c.pos) / c.zoom, color),
        vert!((Vec2::new(c.pos.x, width + y) - c.pos) / c.zoom, color),
        vert!((Vec2::new(c.pos.x + edge, y) - c.pos) / c.zoom, color),
    ];
    vertices
}

pub fn draw_vertical_line(
    x: f32,
    width: f32,
    camera: &Camera,
    config: &Config,
    color: Color,
) -> Vec<Vertex> {
    let edge = camera.zoom * 5.;
    let c = &world_camera(camera, config);
    let r = camera.aspect_ratio();
    let vertices: Vec<Vertex> = vec![
        vert!((Vec2::new(x, c.pos.y - edge) - c.pos) / c.zoom * r, color),
        vert!((Vec2::new(width + x, c.pos.y) - c.pos) / c.zoom * r, color),
        vert!((Vec2::new(x, c.pos.y + edge) - c.pos) / c.zoom * r, color),
    ];
    vertices
}

pub fn add_offseted_indices(src: &mut Vec<u32>, dst: &mut Vec<u32>) {
    if dst.len() == 0 {
        dst.append(src);
        return;
    }
    let mut highest = 0;
    dst.iter().for_each(|s| highest = highest.max(*s));
    highest += 1;
    for idx in &mut *src {
        *idx += highest;
    }
    dst.append(src);
}

pub fn draw_points_and_kites(
    config: &Config,
    camera: &Camera,
    input: &InputStates,
    edit_mode: &EditMode,
    temp_arm: &mut Armature,
    selected_bone_ids: Vec<i32>,
    selections: &SelectionState,
    renderer: &mut Renderer,
    queue: &wgpu::Queue,
    render_pass: &mut RenderPass,
    events: &mut EventState,
    armature: &Armature,
) {
    let point_color: Color = config.colors.center_point;
    let mut kite_color: Color = config.colors.center_point;
    kite_color.a -= 128;
    let in_point_color: Color = config.colors.inactive_center_point;
    let mut in_kite_color: Color = config.colors.inactive_center_point;
    in_kite_color.a = in_kite_color.a.saturating_sub(64);
    let cam = world_camera(&camera, &config);
    let zero = Vec2::default();
    let mut kite_verts = vec![];
    let mut kite_indices = vec![];
    let mut point_verts = vec![];
    let mut point_indices = vec![];
    let mut point_vert_pack_idx = 0;
    let mut kite_vert_pack_idx = 0;
    let mut on_point = false;
    for p in 0..temp_arm.bones.len() {
        let bone = &temp_arm.bones[p];

        // skip points & kites for this bone if editing its mesh
        if edit_mode.showing_mesh && selected_bone_ids.contains(&bone.id) {
            continue;
        }

        let mut color;
        if renderer.render_points || selections.bone_ids.contains(&bone.id) {
            if bone.group_color.a == 0 {
                color = in_point_color;
                if selected_bone_ids.contains(&bone.id) {
                    color = point_color
                }
            } else {
                color = bone.group_color.into();
                if !selected_bone_ids.contains(&bone.id) {
                    color.a /= 2;
                }
            }

            // play shrinking animation if this bone was just selected
            let fade_speed = 0.1;
            let sel_size = config.center_point_radius * 4.;
            let normal_size = config.center_point_radius;
            let elapsed = if selections.bone_ids.len() > 1 && selections.bone_ids.contains(&bone.id)
            {
                (sel_size - edit_mode.sel_time * fade_speed).max(normal_size)
            } else {
                normal_size
            } * camera.zoom;

            let (mut this_verts, mut this_indices) = draw_point(
                &zero, &camera, &config, &bone.pos, color, cam.pos, 0., elapsed,
            );

            // bones can be selected by clicking on their point
            let mouse_on_it = utils::in_bounding_box(&input.mouse, &this_verts, &camera.window).1;
            if mouse_on_it && !camera.on_ui {
                color = bone.group_color.into();
                if bone.group_color.a == 0 {
                    color = point_color;
                }
                if selected_bone_ids.contains(&bone.id) {
                    color += Color::new(64, 64, 64, 255);
                }
                (this_verts, this_indices) = draw_point(
                    &zero, &camera, &config, &bone.pos, color, cam.pos, 0., elapsed,
                );
                // select this bone if point is pressed, unless it was already selected
                if input.left_pressed {
                    let sel_bone = armature.sel_bone(selections);
                    if sel_bone == None {
                        events.select_bone(bone.id as usize, true);
                    } else if sel_bone != None && sel_bone.unwrap().id != bone.id {
                        events.select_bone(bone.id as usize, true);
                    }
                }
                on_point = true;
            }

            for idx in &mut this_indices {
                *idx += point_vert_pack_idx as u32 * 4;
            }

            point_verts.append(&mut this_verts);
            point_indices.append(&mut this_indices.clone());
            point_vert_pack_idx += 1;
        }

        if !renderer.render_kites {
            continue;
        }

        let parent = temp_arm.bones.iter().find(|b| b.id == bone.parent_id);
        if parent == None {
            continue;
        }
        let parent_pos = parent.unwrap().pos;
        let parent_id = parent.unwrap().id;
        let group_color = parent.unwrap().group_color;

        let diff = parent_pos - bone.pos;
        let kite_width = diff.mag() / 1.;
        let kite_rot = diff.y.atan2(diff.x);
        // skip 0 width kites, caused by the child being directly on the parent
        if kite_width == 0. {
            continue;
        }

        if group_color.a == 0 {
            color = in_point_color;
            if selected_bone_ids.contains(&parent_id) {
                color = point_color
            }
        } else {
            color = group_color.into();
            if !selected_bone_ids.contains(&parent_id) {
                color.a /= 2;
            }
        }
        #[rustfmt::skip]
        let (mut this_verts, mut this_indices) = draw_flow_kite(
            &zero, &camera, &config, &temp_arm.bones[p].pos, color, cam.pos, kite_rot, kite_width
        );
        for idx in &mut this_indices {
            *idx += kite_vert_pack_idx * 4;
        }
        kite_verts.append(&mut this_verts);
        kite_indices.append(&mut this_indices);
        kite_vert_pack_idx += 1;
    }
    if kite_indices.len() > 0 {
        render_pass.set_bind_group(0, &renderer.flow_kite_bindgroup, &[]);
        setup_render_buffer(&mut renderer.kite_buffer, &kite_verts, &kite_indices, queue);
        draw(&renderer.kite_buffer, render_pass, 0, kite_indices.len());
    }
    if point_indices.len() > 0 {
        render_pass.set_bind_group(0, &renderer.circle_bindgroup, &[]);
        let point_buffer = &mut renderer.point_buffer;
        setup_render_buffer(point_buffer, &point_verts, &point_indices, queue);
        draw(&renderer.point_buffer, render_pass, 0, point_indices.len());
    }

    if !renderer.on_point {
        renderer.on_point = on_point;
    }
}

// Add vertex and index data to the specified buffer.
fn setup_render_buffer(
    buffer: &RenderBuffer,
    verts: &Vec<Vertex>,
    indices: &Vec<u32>,
    queue: &wgpu::Queue,
) {
    let gpu_verts: Vec<GpuVertex> = verts.iter().map(|vert| (*vert).into()).collect();
    let index_buffer = &buffer.index.as_ref().unwrap();
    let vertex_buffer = &buffer.vertex.as_ref().unwrap();
    queue.write_buffer(index_buffer, 0, bytemuck::cast_slice(&indices));
    queue.write_buffer(vertex_buffer, 0, bytemuck::cast_slice(&gpu_verts));
}

fn transform_ring(
    config: &Config,
    camera: &Camera,
    armature: &Armature,
    temp_arm: &mut Armature,
    render_pass: &mut RenderPass,
    renderer: &mut Renderer,
    events: &mut EventState,
    edit_mode: &EditMode,
    sel: &SelectionState,
    queue: &wgpu::Queue,
    mouse_pos: &Vec2,
) {
    // initiate temporary edit mode if mouse is close enough to bone
    let distance_move = 0.02;
    let distance_rot = config.transform_rot_radius;
    let distance_scale = config.transform_scale_radius;
    let mut temporary = 3;

    render_pass.set_bind_group(0, &renderer.ring_bindgroup, &[]);

    let id = armature.sel_bone(&sel).unwrap().id;
    let sel_bone = temp_arm.bones.iter().find(|b| b.id == id).unwrap();
    let adjusted = Vec2::new(sel_bone.pos.x - mouse_pos.x, sel_bone.pos.y - mouse_pos.y);
    let expand_time = 0.15;
    let size_elapsed = (edit_mode.sel_time / expand_time).min(1.);
    let mut on_point = false;

    // set temporary mode based on distance from bone to cursor
    if !camera.on_ui && size_elapsed == 1. {
        let distance = adjusted.mag() / camera.zoom;

        // prioritize rot or scale, depending on user-defined distance
        if distance < distance_move {
            on_point = true;
            temporary = 0;
            events.set_temporary_edit_mode(0);
        } else if distance_scale > distance_rot {
            if distance < distance_rot {
                on_point = true;
                temporary = 1;
                events.set_temporary_edit_mode(1);
            } else if distance < distance_scale {
                on_point = true;
                temporary = 2;
                events.set_temporary_edit_mode(2);
            }
        } else {
            if distance < distance_scale {
                on_point = true;
                temporary = 2;
                events.set_temporary_edit_mode(2);
            } else if distance < distance_rot {
                on_point = true;
                temporary = 1;
                events.set_temporary_edit_mode(1);
            }
        }
    }

    if !renderer.on_point {
        renderer.on_point = on_point;
    }

    // draw rot and scale rings
    let cam = world_camera(&camera, &config);
    let mut rot_col = config.colors.transform_rings;
    let mut scale_col = config.colors.transform_rings;
    let rot_radius = distance_rot * camera.zoom * size_elapsed;
    let scale_radius = distance_scale * camera.zoom * size_elapsed;
    scale_col -= Color::new(25, 25, 25, 0);
    #[rustfmt::skip]
    let (mut verts_rot, mut indices_rot) = draw_point(&Vec2::ZERO, camera, config, &sel_bone.pos, rot_col, cam.pos, 0., rot_radius);
    #[rustfmt::skip]
    let (mut verts_scale, mut indices_scale) = draw_point(&Vec2::ZERO, camera, config, &sel_bone.pos, scale_col, cam.pos, 0., scale_radius);
    verts_rot.extend_from_slice(&mut verts_scale);
    add_offseted_indices(&mut indices_scale, &mut indices_rot);
    setup_render_buffer(&mut renderer.ring_buffer, &verts_rot, &indices_rot, queue);
    draw(&renderer.ring_buffer, render_pass, 0, indices_rot.len());

    // setup rot background
    if temporary != 1 {
        rot_col -= Color::new(0, 0, 0, 200);
    } else {
        rot_col -= Color::new(0, 0, 0, 150);
    }
    render_pass.set_bind_group(0, &renderer.circle_bindgroup, &[]);
    #[rustfmt::skip]
    let (mut sel_verts, mut sel_indices) = draw_point(&Vec2::ZERO, camera, config, &sel_bone.pos, rot_col, cam.pos, 0., rot_radius);

    // setup scale background
    if temporary != 2 {
        scale_col -= Color::new(0, 0, 0, 200);
    } else {
        scale_col -= Color::new(0, 0, 0, 150);
    }
    render_pass.set_bind_group(0, &renderer.circle_bindgroup, &[]);
    #[rustfmt::skip]
    let (mut sel_verts1, mut sel_indices1) = draw_point(&Vec2::ZERO, camera, config, &sel_bone.pos, scale_col, cam.pos, 0., scale_radius);

    // draw both backgrounds
    sel_verts.append(&mut sel_verts1);
    add_offseted_indices(&mut sel_indices1, &mut sel_indices);
    let buffer = &mut renderer.selected_ring_buffer;
    setup_render_buffer(buffer, &sel_verts, &sel_indices, queue);
    draw(&buffer, render_pass, 0, indices_rot.len());

    // set temporary mode to None only once, to prevent event spam
    if temporary == 3 && edit_mode.temporary != None {
        events.set_temporary_edit_mode(3);
    }
}
