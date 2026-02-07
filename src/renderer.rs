//! Core rendering logic, abstracted from the rest of WGPU.

use crate::*;
use image::{DynamicImage, GenericImageView};
use spade::Triangulation;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};

/// The `main` of this module.
pub fn render(
    render_pass: &mut RenderPass,
    device: &Device,
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

    #[cfg(target_arch = "wasm32")]
    if !renderer.has_loaded {
        loaded();
        renderer.has_loaded = true;
    }

    // create vert on cursor
    let space = utils::screen_to_world_space(input.mouse, camera.window);
    let mut mouse_world_vert = vert(Some(space), None, None);
    mouse_world_vert.pos.x *= camera.window.y / camera.window.x;

    if !config.gridline_front {
        draw_gridline(render_pass, device, &renderer, &camera, &config);
    }

    let mut temp_arm = armature.clone();
    let mut anim_bones = armature.animated_bones.clone();

    // adjust anim_bones' verts for new textrues mid-animations
    temp_arm.bones = anim_bones.clone();
    for b in 0..armature.bones.len() {
        let tex = temp_arm.tex_of(armature.bones[b].id);
        if !armature.bones[b].verts_edited && tex != None {
            let size = tex.unwrap().size;
            (anim_bones[b].vertices, anim_bones[b].indices) = create_tex_rect(&size);
        }
    }

    temp_arm.bones = anim_bones.clone();

    // store bound/unbound vert's pos before construction
    let mut init_vert_pos = Vec2::default();
    let vert_id = renderer.changed_vert_id as usize;
    if renderer.changed_vert_id != -1 {
        init_vert_pos = temp_arm.bones[selections.bone_idx].vertices[vert_id].pos;
    }

    construction(&mut temp_arm.bones, &anim_bones);

    // adjust bound/unbound vert's pos after construction
    if renderer.changed_vert_id != -1 {
        let temp_vert = temp_arm.bones[selections.bone_idx].vertices[vert_id];

        let mut diff = temp_vert.pos - init_vert_pos - temp_arm.bones[selections.bone_idx].pos;

        // if unbound, vert needs to account for pos in the previous frame
        let vert = &armature.sel_bone(&sel).unwrap().vertices[vert_id];
        if let Some(last_frame_pos) = renderer.changed_vert_init_pos {
            diff = temp_vert.pos - last_frame_pos;
        }

        events.adjust_vertex(vert.pos.x - diff.x, vert.pos.y - diff.y);
    }

    let mut mesh_onion_id = -1;

    let mut selected_bones_pos = vec![];
    if armature.sel_bone(&sel) != None {
        let id = armature.sel_bone(&sel).unwrap().id;
        let bone = temp_arm.bones.iter().find(|bone| bone.id == id).unwrap();
        let mut children = vec![bone.clone()];
        armature_window::get_all_children(&temp_arm.bones, &mut children, &bone);
        for child in children {
            selected_bones_pos.push(child.pos);
        }
    }

    // sort bones by highest zindex first, so that hover logic will pick the top-most one
    temp_arm.bones.sort_by(|a, b| b.zindex.cmp(&a.zindex));

    let mut hover_bone_id = -1;

    // many fight for spot of newest vertex; only one will emerge victorious.
    let mut new_vert: Option<Vertex> = None;
    let mut removed_vert = false;

    // pre-draw bone setup
    for b in 0..temp_arm.bones.len() {
        let tex = temp_arm.tex_of(temp_arm.bones[b].id);
        let parents = armature.get_all_parents(temp_arm.bones[b].id);

        let selected_bone = armature.sel_bone(&sel);
        if selected_bone != None && selected_bone.unwrap().id == temp_arm.bones[b].id {
            for parent in &parents {
                let tex = temp_arm.tex_of(parent.id);
                if tex != None && parent.verts_edited {
                    mesh_onion_id = parent.id;
                    break;
                }
            }
        }

        if tex == None || temp_arm.bones[b].is_hidden {
            continue;
        }

        // save constructed vertices for the ClickVertex event
        if selections.bone_idx != usize::MAX
            && temp_arm.bones[b].id == armature.sel_bone(&sel).unwrap().id
        {
            renderer.sel_temp_bone = Some(temp_arm.bones[b].clone());
        }

        let cam = world_camera(&camera, &config);
        for v in 0..temp_arm.bones[b].vertices.len() {
            let tb = &mut temp_arm.bones[b];
            let mut vert = world_vert(tb.vertices[v], &cam, camera.aspect_ratio(), Vec2::default());
            vert.tint = tb.tint;
            tb.world_verts.push(vert);
        }

        for vert in &mut temp_arm.bones[b].world_verts {
            vert.add_color = VertexColor::new(0., 0., 0., 0.);
        }
        if edit_mode.setting_bind_verts {
            continue;
        }

        // check if cursor is on an opaque pixel of this bone's texture
        let tb = &temp_arm.bones[b];
        let selected_mesh = !edit_mode.showing_mesh
            || edit_mode.showing_mesh && armature.sel_bone(&sel).unwrap().id == tb.id;
        if hover_bone_id == -1 && !input.left_down && !camera.on_ui && selected_mesh {
            let wv = &temp_arm.bones[b].world_verts;
            for (i, chunk) in temp_arm.bones[b].indices.chunks_exact(3).enumerate() {
                let c0 = chunk[0] as usize;
                let c1 = chunk[1] as usize;
                let c2 = chunk[2] as usize;

                let bary = tri_point(&mouse_world_vert.pos, &wv[c0].pos, &wv[c1].pos, &wv[c2].pos);
                if bary.0 == -1. {
                    continue;
                }

                let bones = &temp_arm.bones;
                let v = &bones.iter().find(|bone| bone.id == tb.id).unwrap().vertices;
                let uv = v[c0].uv * bary.3 + v[c1].uv * bary.1 + v[c2].uv * bary.2;
                let mut pos = (utils::rotate(&v[c0].pos, -tb.rot) - tb.pos) * bary.3
                    + (utils::rotate(&v[c1].pos, -tb.rot) - tb.pos) * bary.1
                    + (utils::rotate(&v[c2].pos, -tb.rot) - tb.pos) * bary.2;

                if edit_mode.showing_mesh && input.right_clicked && !removed_vert {
                    if armature.sel_bone(&sel).unwrap().indices.len() == 6 {
                        events.open_modal("indices_limit", false);
                    } else {
                        events.remove_triangle(i * 3);
                        removed_vert = true;
                    }
                    break;
                }

                if edit_mode.showing_mesh && input.left_clicked && new_vert == None {
                    pos /= tb.scale;
                    new_vert = Some(vert(Some(pos), None, Some(uv)));
                    break;
                }

                let tex = temp_arm.tex_of(temp_arm.bones[b].id).unwrap();
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
        if hover_bone_id == temp_arm.bones[b].id
            && (idx == usize::MAX || armature.bones[idx].id != click_on_hover_id)
        {
            let fade = 0.25 * ((edit_mode.time * 3.).sin()).abs() as f32;
            let min = 0.1;
            for vert in &mut temp_arm.bones[b].world_verts {
                vert.add_color = VertexColor::new(min + fade, min + fade, min + fade, 0.);
            }
        } else {
            for vert in &mut temp_arm.bones[b].world_verts {
                vert.add_color = VertexColor::new(0., 0., 0., 0.);
            }
        }

        // select bone on click
        if input.left_clicked && hover_bone_id == temp_arm.bones[b].id {
            let id = temp_arm.bones[b].id;
            let bones = &armature.bones;
            let idx = bones.iter().position(|bone| bone.id == id).unwrap();
            events.select_bone(idx, true);
        }
    }

    renderer.temp_bones = temp_arm.bones.clone();

    // runtime: sort bones by z-index for drawing
    temp_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    for b in 0..temp_arm.bones.len() {
        let tex = temp_arm.tex_of(temp_arm.bones[b].id);
        if tex == None || temp_arm.bones[b].is_hidden {
            continue;
        }

        if edit_mode.showing_mesh && armature.sel_bone(&sel).unwrap().id == temp_arm.bones[b].id {
            continue;
        }

        let t = tex.unwrap();
        let bg = armature.tex_data(t).unwrap().bind_group.clone();
        let bone = &temp_arm.bones[b];
        draw(&bg, &bone.world_verts, &bone.indices, render_pass, device);
    }

    // draw inverse kinematics arrows

    // todo:
    // only draw arrows for the selected set of bones.
    // currently it shows all when any are selected.
    let sel_bone = armature.sel_bone(&sel);
    if sel_bone != None && armature.bone_eff(sel_bone.unwrap().id) != JointEffector::None {
        for bone in &temp_arm.bones {
            let bone_eff = armature.bone_eff(bone.id);
            if bone_eff == JointEffector::None || bone_eff == JointEffector::End {
                continue;
            }
            let mut arrow = Bone {
                pos: bone.pos,
                rot: bone.rot,
                scale: Vec2::new(2., 2.),
                ..Default::default()
            };
            let size = Vec2::new(61., 48.);
            (arrow.vertices, arrow.indices) = create_tex_rect(&size);
            let ratio = camera.aspect_ratio();
            let pivot = Vec2::new(0., 0.5);
            for v in 0..4 {
                let verts = arrow.vertices[v];
                let mut new_vert = world_vert(verts, &camera, ratio, pivot);
                new_vert.color = VertexColor::new(1., 1., 1., 0.2);
                arrow.world_verts.push(new_vert);
            }
            let bg = &renderer.ik_arrow_bindgroup;
            draw(bg, &arrow.world_verts, &arrow.indices, render_pass, device);
        }
    }

    if edit_mode.showing_mesh || edit_mode.setting_bind_verts {
        let id = armature.sel_bone(&sel).unwrap().id;
        let bone = temp_arm.bones.iter().find(|bone| bone.id == id).unwrap();
        let tex = temp_arm.tex_of(bone.id).unwrap();
        let bind_group = &armature.tex_data(tex).unwrap().bind_group;
        let verts = &bone.world_verts;
        draw(&bind_group, &verts, &bone.indices, render_pass, device);
        render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);
        let mouse = mouse_world_vert;
        let nw = &mut new_vert;
        let wv = bone.world_verts.clone();

        #[rustfmt::skip]
        let (verts, indices, on_vert) = bone_vertices(&wv, true, selections, input, camera, config, edit_mode, events, armature, renderer);
        #[rustfmt::skip]
        let (lines_v, lines_i, on_line) = vert_lines(bone, &temp_arm.bones, &mouse, nw, true, on_vert, camera, input, renderer);

        draw(&None, &lines_v, &lines_i, render_pass, device);
        draw(&None, &verts, &indices, render_pass, device);

        // draw hovered triangle if neither a vertex nor a line is hovered
        let mut hovering_tri = bone_triangle(&bone, &mouse, wv);
        if hovering_tri.len() > 0 && !on_vert && !on_line {
            hovering_tri[0].add_color = VertexColor::new(-255., 0., -255., -0.75);
            hovering_tri[1].add_color = VertexColor::new(-255., 0., -255., -0.75);
            hovering_tri[2].add_color = VertexColor::new(-255., 0., -255., -0.75);
            draw(&None, &hovering_tri, &vec![0, 1, 2], render_pass, device);

            // verts of this triangle will be dragged
            if input.left_pressed {
                renderer.dragging_verts = hovering_tri.iter().map(|v| v.id as usize).collect();
            }
        }
    }

    if mesh_onion_id != -1 {
        render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);
        let tp = &temp_arm.bones;
        let bone = tp.iter().find(|bone| bone.id == mesh_onion_id).unwrap();
        let wv = bone.world_verts.clone();
        let vertex = Vertex::default();

        #[rustfmt::skip]
        let (verts, indices, _) = vert_lines(bone, &tp, &vertex, &mut None, true, false, camera, input, renderer);
        draw(&None, &verts, &indices, render_pass, device);

        #[rustfmt::skip]
        let (verts, indices, _) = bone_vertices(&wv, false, selections, input, camera, config, edit_mode, events, armature, renderer);
        draw(&None, &verts, &indices, render_pass, device);
    }

    if !edit_mode.setting_bind_verts {
        if new_vert != None {
            renderer.new_vert = new_vert;
            events.new_vertex();
        }
    }

    if config.gridline_front {
        draw_gridline(render_pass, device, &renderer, &camera, &config);
    }

    render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);

    let mut color = VertexColor::new(
        config.colors.center_point.r as f32 / 255.,
        config.colors.center_point.g as f32 / 255.,
        config.colors.center_point.b as f32 / 255.,
        0.75,
    );
    let cam = world_camera(&camera, &config);
    let zero = Vec2::default();
    let mut point_verts = vec![];
    let mut point_indices = vec![];
    for p in 0..selected_bones_pos.len() {
        if p > 0 {
            color.a = 0.25;
        }
        let pos = selected_bones_pos[p];
        let (mut this_verts, mut this_indices) =
            draw_point(&zero, &camera, &config, &pos, color, cam.pos, 0.);
        for idx in &mut this_indices {
            *idx += p as u32 * 4;
        }
        point_verts.append(&mut this_verts);
        point_indices.append(&mut this_indices);
    }
    if point_indices.len() > 0 {
        draw(&mut None, &point_verts, &point_indices, render_pass, device);
    }

    if !input.left_down {
        renderer.dragging_verts = vec![];
        renderer.editing_bone = false;
        renderer.started_dragging_verts = false;
    } else if renderer.dragging_verts.len() > 0 {
        if !renderer.started_dragging_verts {
            events.save_bone(selections.bone_idx);
            renderer.started_dragging_verts = true
        }
        for vert_id in renderer.dragging_verts.clone() {
            events.drag_vertex(vert_id);
        }

        return;
    }

    if input.mouse_init == None {
        if let Some(bone) = armature.sel_bone(&sel) {
            let sel_anim_bone = temp_arm.bones.iter().find(|b| b.id == bone.id).unwrap();
            renderer.bone_init_rot = sel_anim_bone.rot;
        }
    }

    if !input.left_down && !input.right_down {
        return;
    }

    // move camera
    if (input.holding_mod || input.right_down) && !camera.on_ui {
        let vel = renderer::mouse_vel(&input, &camera) * camera.zoom;
        events.edit_camera(camera.pos.x + vel.x, camera.pos.y + vel.y, camera.zoom);
        return;
    }

    let mut ik_disabled = true;
    if let Some(bone) = armature.sel_bone(&sel) {
        let is_end = edit_mode.current == EditModes::Rotate
            && armature.bone_eff(bone.id) == JointEffector::End;
        ik_disabled =
            is_end || (bone.ik_disabled || armature.bone_eff(bone.id) == JointEffector::None);
    }

    if edit_mode.showing_mesh || !ik_disabled {
        return;
    }

    // editing bone
    let idx = sel.bone_idx;
    let input = &input;
    if camera.on_ui {
        renderer.editing_bone = false;
    } else if idx != usize::MAX && input.left_down && hover_bone_id == -1 && input.down_dur > 5 {
        if edit_mode.current == EditModes::Rotate {
            let mut mouse = utils::screen_to_world_space(input.mouse, camera.window);
            mouse.x *= camera.aspect_ratio();
            let id = armature.sel_bone(&sel).unwrap().id;
            let bone = temp_arm.bones.iter().find(|b| b.id == id).unwrap();
            let center = vert(Some(bone.pos), None, None);
            let cam = &world_camera(&camera, &config);
            let aspect_ratio = camera.aspect_ratio();
            let cw = world_vert(center, cam, aspect_ratio, Vec2::new(0.5, 0.5));
            render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);
            draw_line(cw.pos, mouse, render_pass, &device);
        }

        if !renderer.editing_bone {
            events.save_edited_bone(selections.bone_idx);
            renderer.editing_bone = true;
        }

        let id = armature.sel_bone(&sel).unwrap().id;
        let bone = temp_arm.bones.iter().find(|b| b.id == id).unwrap();

        #[rustfmt::skip]
        edit_bone(events, edit_mode, &selections, &camera, &config, &input, &renderer, bone, &temp_arm.bones);
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

fn vert(pos: Option<Vec2>, col: Option<VertexColor>, uv: Option<Vec2>) -> Vertex {
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

/// Stripped-down renderer for screenshot purposes.
pub fn render_screenshot(
    render_pass: &mut RenderPass,
    device: &Device,
    armature: &Armature,
    camera: &Camera,
    config: &Config,
) {
    let mut temp_arm = Armature::default();
    temp_arm.bones = armature.bones.clone();
    construction(&mut temp_arm.bones, &armature.bones);
    temp_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    let mut cam = world_camera(&camera, &config).clone();
    cam.pos = Vec2::new(0., 0.);
    cam.zoom = 1500.;

    for b in 0..temp_arm.bones.len() {
        if armature.tex_of(temp_arm.bones[b].id) == None {
            continue;
        }
        if temp_arm.bones[b].is_hidden {
            continue;
        }

        for v in 0..temp_arm.bones[b].vertices.len() {
            let tb = &temp_arm.bones[b];
            let mut new_vert =
                world_vert(tb.vertices[v], &cam, camera.aspect_ratio(), Vec2::default());
            new_vert.add_color = VertexColor::new(0., 0., 0., 0.);
            temp_arm.bones[b].world_verts.push(new_vert);
        }

        let arm = &armature;
        let id = temp_arm.bones[b].id;
        let tex = arm.tex_of(id).unwrap();
        let bg = &armature.tex_data(tex).unwrap().bind_group;
        let bones = &temp_arm.bones[b];
        draw(bg, &bones.world_verts, &bones.indices, render_pass, device);
    }
}

pub fn construction(bones: &mut Vec<Bone>, og_bones: &Vec<Bone>) {
    inheritance(bones, std::collections::HashMap::new());

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
    inheritance(bones, ik_rot.clone());

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
            let bind_bone = bones.iter().find(|bone| bone.id == b_id).unwrap().clone();
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
                let binds = &bones[b].binds;
                let prev = if bi > 0 { bi - 1 } else { bi };
                let next = (bi + 1).min(binds.len() - 1);
                if binds[prev].bone_id == -1 || binds[next].bone_id == -1 {
                    continue;
                }
                let prev_bone = bones.iter().find(|bone| bone.id == binds[prev].bone_id);
                let next_bone = bones.iter().find(|bone| bone.id == binds[next].bone_id);

                // get the average of normals between previous bone, this bone, and next bone
                let prev_dir = bind_bone.pos - prev_bone.unwrap().pos;
                let next_dir = next_bone.unwrap().pos - bind_bone.pos;
                let prev_normal = Vec2::new(-prev_dir.y, prev_dir.x).normalize();
                let next_normal = Vec2::new(-next_dir.y, next_dir.x).normalize();
                let average = prev_normal + next_normal;
                let normal_angle = average.y.atan2(average.x);

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
        if b == bones.len() - 1 {
            continue;
        }

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
            root.x - (root.x - bones[b].pos.x) * valley,
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
        let mut length = (next_pos - bones[b].pos).normalize() * next_length;
        if length.x.is_nan() {
            length = Vec2::new(0., 0.);
        }
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
        let mut length = (prev_pos - bones[b].pos).normalize() * prev_length;
        if length.x.is_nan() {
            length = Vec2::new(0., 0.);
        }
        if b != bones.len() - 1 {
            prev_length = (bones[b].pos - bones[b + 1].pos).mag();
        }
        bones[b].pos = prev_pos - length;
        prev_pos = bones[b].pos;
    }
}

/// sort vertices in cw (or ccw?) order
pub fn sort_vertices(mut verts: Vec<Vertex>) -> Vec<Vertex> {
    let mut center = Vec2::default();
    for v in 0..verts.len() {
        center += verts[v].pos;
    }
    center /= verts.len() as f32;

    verts.sort_by(|a, b| {
        let angle_a = (a.pos.y - center.y).atan2(a.pos.x - center.x);
        let angle_b = (b.pos.y - center.y).atan2(b.pos.x - center.x);
        angle_a.partial_cmp(&angle_b).unwrap()
    });

    verts
}

fn winding_sort(mut points: Vec<Vec2>) -> Vec<Vec2> {
    let mut center = Vec2::default();
    for p in &points {
        center += *p;
    }
    center /= points.len() as f32;

    points.sort_by(|a, b| {
        let angle_a = (a.y - center.y).atan2(a.x - center.x);
        let angle_b = (b.y - center.y).atan2(b.x - center.x);
        angle_a.partial_cmp(&angle_b).unwrap()
    });

    points
}

pub fn edit_bone(
    events: &mut EventState,
    edit_mode: &EditMode,
    selections: &SelectionState,
    camera: &Camera,
    config: &Config,
    input: &InputStates,
    renderer: &Renderer,
    bone: &Bone,
    bones: &Vec<Bone>,
) {
    let mut anim_id = selections.anim;
    let anim_frame = selections.anim_frame;
    if !edit_mode.anim_open {
        anim_id = usize::MAX;
    }

    macro_rules! edit {
        ($bone:expr, $element:expr, $value:expr) => {
            events.edit_bone($bone.id, &$element, $value, anim_id, anim_frame);
        };
    }

    let vert = vert(Some(bone.pos), None, None);
    let cam = &world_camera(&camera, &config);
    let bone_center = world_vert(vert, cam, camera.aspect_ratio(), Vec2::new(0.5, 0.5));

    if edit_mode.current == EditModes::Move {
        let mut pos = bone.pos;
        let mouse_vel = mouse_vel(&input, &camera) * camera.zoom;

        // move position with mouse velocity
        pos -= mouse_vel;

        // restore universal position by offsetting against parents' attributes
        if bone.parent_id != -1 {
            let parent = bones.iter().find(|b| b.id == bone.parent_id).unwrap();
            pos -= parent.pos;
            pos = utils::rotate(&pos, -parent.rot);
            pos /= parent.scale;
        }

        edit!(bone, AnimElement::PositionX, pos.x);
        edit!(bone, AnimElement::PositionY, pos.y);
    } else if edit_mode.current == EditModes::Rotate {
        let mouse_init = utils::screen_to_world_space(input.mouse_init.unwrap(), camera.window);
        let dir_init = mouse_init - bone_center.pos;
        let rot_init = dir_init.y.atan2(dir_init.x);

        let mouse = utils::screen_to_world_space(input.mouse, camera.window);
        let dir = mouse - bone_center.pos;
        let rot = dir.y.atan2(dir.x);

        let rot = renderer.bone_init_rot + (rot - rot_init);
        edit!(bone, AnimElement::Rotation, rot);
    } else if edit_mode.current == EditModes::Scale {
        let mut scale = bone.scale;

        // restore universal scale, by offsetting against parent's
        if bone.parent_id != -1 {
            let parent = bones.iter().find(|b| b.id == bone.parent_id).unwrap();
            scale /= parent.scale;
        }

        scale -= mouse_vel(&input, &camera);

        edit!(bone, AnimElement::ScaleX, scale.x);
        edit!(bone, AnimElement::ScaleY, scale.y);
    }
}

pub fn inheritance(bones: &mut Vec<Bone>, ik_rot: std::collections::HashMap<i32, f32>) {
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

            // rotate such that it will orbit the parent
            bones[i].pos = utils::rotate(&bones[i].pos, parent.rot);

            // inherit position from parent
            bones[i].pos += parent.pos;
        }

        // apply rotations from IK, if provided
        let ik_rot = ik_rot.get(&bones[i].id);
        if ik_rot != None {
            bones[i].rot = *ik_rot.unwrap();
        }
    }
}

pub fn draw(
    bind_group: &Option<BindGroup>,
    verts: &Vec<Vertex>,
    indices: &Vec<u32>,
    render_pass: &mut RenderPass,
    device: &Device,
) {
    if *bind_group != None {
        render_pass.set_bind_group(0, bind_group, &[]);
    }
    render_pass.set_vertex_buffer(0, vertex_buffer(&verts, device).slice(..));
    render_pass.set_index_buffer(
        index_buffer(indices.to_vec(), &device).slice(..),
        wgpu::IndexFormat::Uint32,
    );
    render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
}

pub fn bone_vertices(
    world_verts: &Vec<Vertex>,
    editable: bool,
    selections: &SelectionState,
    input: &InputStates,
    camera: &Camera,
    config: &Config,
    edit_mode: &EditMode,
    events: &mut EventState,
    armature: &Armature,
    renderer: &mut Renderer,
) -> (Vec<Vertex>, Vec<u32>, bool) {
    let mut all_verts = vec![];
    let mut all_indices = vec![];
    let mut hovering_vert = false;
    let v2z = Vec2::ZERO;
    let rotated = 45. * 3.14 / 180.;
    let sel = selections.clone();
    #[rustfmt::skip]
    macro_rules! point {
        ($idx:expr, $color:expr) => {
            draw_point(&world_verts[$idx].pos, &camera, &config, &v2z, $color, v2z, rotated)
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
        let idx = selections.bind;
        let verts: Vec<i32>;
        if idx == -1 {
            verts = vec![];
        } else {
            let selected = armature.sel_bone(&sel).unwrap();
            verts = selected.binds[idx as usize]
                .verts
                .iter()
                .map(|v| v.id)
                .collect();
        }

        let bound = idx != -1 && verts.contains(&(world_verts[wv].id as i32));
        type Vc = VertexColor;
        let mut col = if bound { Vc::YELLOW } else { Vc::GREEN };
        col.a = if editable { 0.5 } else { 0.15 };
        let (mut verts, mut indices) = point!(wv, col);
        let mouse_on_it = utils::in_bounding_box(&input.mouse, &verts, &camera.window).1;

        if camera.on_ui || !mouse_on_it || !editable {
            add_point!(verts, indices, wv);
            continue;
        }

        hovering_vert = true;

        let (mut verts, mut indices) = point!(wv, VertexColor::WHITE);
        add_point!(verts, indices, wv);
        if input.right_clicked {
            if world_verts.len() <= 4 {
                events.open_modal("vert_limit", false);
            } else {
                events.remove_vertex(wv);
                break;
            }
        }
        if !edit_mode.setting_bind_verts {
            if input.left_pressed {
                renderer.dragging_verts = vec![world_verts[wv].id as usize];
                break;
            }
        } else if input.left_clicked {
            events.click_vertex(wv);
            break;
        }
    }

    (all_verts, all_indices, hovering_vert)
}

fn bone_triangle(tb: &Bone, mouse_world_vert: &Vertex, wv: Vec<Vertex>) -> Vec<Vertex> {
    let mut hovering_tri = vec![];
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
    }

    hovering_tri
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
    renderer: &mut Renderer,
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

        let mut col = VertexColor::GREEN;
        col += VertexColor::new(-0.5, -0.5, -0.5, 0.);
        col.a = if editable { 0.3 } else { 0.1 };

        #[rustfmt::skip]
        macro_rules! vert { ($pos:expr, $v:expr) => { Vertex { pos: $pos, color: col, ..$v } }; }

        let mut v0_top = vert!(v0.pos + base, v0);
        let mut v0_bot = vert!(v0.pos - base, v0);
        let mut v1_top = vert!(v1.pos + base, v1);
        let mut v1_bot = vert!(v1.pos - base, v1);

        let verts = vec![v0_top, v0_bot, v1_top, v1_bot];
        let add_color = VertexColor::new(0.2, 0.2, 0.2, 1.);

        let mut is_hovering = false;

        if editable && !hovering_vert {
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
                    renderer.dragging_verts.push(verts[i0 as usize].id as usize);
                    renderer.dragging_verts.push(verts[i1 as usize].id as usize);
                } else if input.left_clicked && !added_vert {
                    let bones = &bones;
                    let v = &bones.iter().find(|b| b.id == bone.id).unwrap().vertices;
                    let wv0 = v[i0 as usize].pos - bone.pos;
                    let wv1 = v[i1 as usize].pos - bone.pos;
                    let pos = wv0 + (wv1 - wv0) * interp;
                    *new_vert = Some(vert(Some(pos), None, Some(uv)));
                    added_vert = true;
                }
            }

            if is_hovering {
                v0_top.add_color += add_color;
                v0_bot.add_color += add_color;
                v1_top.add_color += add_color;
                v1_bot.add_color += add_color;
            }

            let mv = &renderer.dragging_verts;

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

fn draw_line(origin: Vec2, target: Vec2, render_pass: &mut RenderPass, device: &Device) {
    let dir = target - origin;

    let width = 2.5;
    let mut base = Vec2::new(width, width) / 1000.;
    base = utils::rotate(&base, dir.y.atan2(dir.x) + (45. * 3.14 / 180.));

    let color = VertexColor::new(0., 1., 0., 1.);

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

    draw(&None, &verts, &indices, render_pass, device);
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
    verts = sort_vertices(verts.clone());
    let indices = vec![0, 1, 2, 0, 2, 3];
    (verts, indices)
}

pub fn trace_mesh(texture: &image::DynamicImage) -> (Vec<Vertex>, Vec<u32>) {
    let gap = 25.;
    let mut poi: Vec<Vec2> = vec![];

    // used to create extra space across the image
    let padding = 50.;

    // place points across the image where it's own pixel is fully transparent
    let mut cursor = Vec2::default();
    while cursor.y < texture.height() as f32 + padding {
        let out_of_bounds = cursor.x > texture.width() as f32 || cursor.y > texture.height() as f32;
        if out_of_bounds || texture.get_pixel(cursor.x as u32, cursor.y as u32).0[3] == 0 {
            poi.push(cursor);
        }
        cursor.x += gap;
        if cursor.x > texture.width() as f32 + padding {
            cursor.x = 0.;
            cursor.y += gap;
        }
    }

    // remove points which have 8 neighbours, keeping only points
    // that are closest to the image
    let poi_clone = poi.clone();
    poi.retain(|point| {
        let left = Vec2::new(point.x - gap, point.y);
        let right = Vec2::new(point.x + gap, point.y);
        let up = Vec2::new(point.x, point.y + gap);
        let down = Vec2::new(point.x, point.y - gap);

        let lt = Vec2::new(point.x - gap, point.y + gap);
        let lb = Vec2::new(point.x - gap, point.y - gap);
        let rt = Vec2::new(point.x + gap, point.y + gap);
        let rb = Vec2::new(point.x + gap, point.y - gap);

        macro_rules! p {
            ($dir:expr) => {
                !poi_clone.contains($dir)
                    && $dir.x > 0.
                    && $dir.y > 0.
                    && $dir.x < texture.width() as f32
                    && $dir.y < texture.height() as f32
            };
        }

        p!(&left) || p!(&right) || p!(&up) || p!(&down) || p!(&lt) || p!(&lb) || p!(&rt) || p!(&rb)
    });

    // sort points in any winding order
    poi = winding_sort(poi);

    let uv_x = poi[0].x / texture.width() as f32;
    let uv_y = poi[0].y / texture.height() as f32;
    let pos = Vec2::new(poi[0].x, -poi[0].y);
    let mut verts = vec![vert(Some(pos), None, Some(Vec2::new(uv_x, uv_y)))];
    let mut curr_poi = 0;

    // get last point that current one has light of sight on
    // if next point checked happens to be first and there's line of sight, tracing is over
    let mut id = 1;
    for p in curr_poi..poi.len() {
        if p == poi.len() - 1 {
            break;
        }
        if line_of_sight(&texture, poi[curr_poi], poi[(p + 1) % (poi.len() - 1)]) {
            continue;
        }
        if p == 0 {
            curr_poi = 1;
            continue;
        }

        let tex = Vec2::new(texture.width() as f32, texture.height() as f32);
        verts.push(Vertex {
            pos: Vec2::new(poi[p - 1].x, -poi[p - 1].y),
            uv: poi[p - 1] / tex,
            id,
            ..Default::default()
        });
        id += 1;
        curr_poi = p - 1;
    }

    //for point in poi {
    //    verts.push(Vertex {
    //        pos: Vec2::new(point.x, -point.y),
    //        uv: Vec2::new(
    //            point.x / texture.width() as f32,
    //            point.y / texture.height() as f32,
    //        ),
    //        ..Default::default()
    //    });
    //}

    verts = sort_vertices(verts);
    editor::center_verts(&mut verts);
    (verts.clone(), triangulate(&verts, texture))
}

fn draw_point(
    offset: &Vec2,
    camera: &Camera,
    config: &Config,
    pos: &Vec2,
    color: VertexColor,
    camera_pos: Vec2,
    rotation: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let point_size = 6. * (camera.zoom / 500.);
    macro_rules! vert {
        ($pos:expr, $uv:expr) => {
            vert(Some($pos), Some(color), Some($uv))
        };
    }
    let mut temp_point_verts: [Vertex; 4] = [
        vert!(Vec2::new(-point_size, point_size), Vec2::new(1., 0.)),
        vert!(Vec2::new(point_size, point_size), Vec2::new(0., 1.)),
        vert!(Vec2::new(-point_size, -point_size), Vec2::new(0., 0.)),
        vert!(Vec2::new(point_size, -point_size), Vec2::new(1., 1.)),
    ];

    for v in &mut temp_point_verts {
        v.pos += *pos;
        v.pos = utils::rotate(&v.pos, rotation);
    }

    let mut point_verts = vec![];
    let ar = camera.aspect_ratio();
    let mut cam = world_camera(&camera, &config).clone();
    cam.pos = camera_pos;
    let pivot = Vec2::new(0.5, 0.5);
    for vert in temp_point_verts {
        let vert = world_vert(vert, &cam, ar, pivot);
        point_verts.push(vert);
    }

    for vert in &mut point_verts {
        vert.pos += *offset;
    }

    let indices = vec![0, 1, 2, 1, 2, 3];

    (point_verts, indices)
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

fn index_buffer(indices: Vec<u32>, device: &Device) -> wgpu::Buffer {
    wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        },
    )
}

fn vertex_buffer(vertices: &Vec<Vertex>, device: &Device) -> wgpu::Buffer {
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

fn world_vert(mut vert: Vertex, camera: &Camera, aspect_ratio: f32, pivot: Vec2) -> Vertex {
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
    device: &Device,
    renderer: &Renderer,
    camera: &Camera,
    config: &Config,
) {
    render_pass.set_bind_group(0, &renderer.generic_bindgroup, &[]);

    let cam = world_camera(camera, config);

    let col = VertexColor::new(
        config.colors.gridline.r as f32 / 255.,
        config.colors.gridline.g as f32 / 255.,
        config.colors.gridline.b as f32 / 255.,
        1.,
    );

    let width = 0.005 * cam.zoom;
    let regular_color = VertexColor::new(col.r, col.g, col.b, 0.15);
    let highlight_color = VertexColor::new(col.r, col.g, col.b, 1.);

    let mut verts = vec![];
    let mut indices: Vec<u32> = vec![];
    let mut i: u32 = 0;

    // draw vertical lines
    let mut x = (cam.pos.x - cam.zoom / camera.aspect_ratio()).round();
    let right_side = cam.pos.x + cam.zoom / camera.aspect_ratio();
    while x < right_side {
        if x % renderer.gridline_gap as f32 != 0. {
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
        if y % renderer.gridline_gap as f32 != 0. {
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

    render_pass.set_index_buffer(
        index_buffer(indices.clone(), &device).slice(..),
        wgpu::IndexFormat::Uint32,
    );
    render_pass.set_vertex_buffer(0, vertex_buffer(&verts, device).slice(..));
    render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
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
    color: VertexColor,
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
    color: VertexColor,
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

pub fn triangulate(verts: &Vec<Vertex>, tex: &image::DynamicImage) -> Vec<u32> {
    let mut triangulation: spade::DelaunayTriangulation<_> = spade::DelaunayTriangulation::new();

    for vert in verts {
        let _ = triangulation.insert(spade::Point2::new(vert.uv.x, vert.uv.y));
    }

    let mut indices: Vec<u32> = Vec::new();
    for face in triangulation.inner_faces() {
        let tri_indices = face.vertices().map(|v| v.index()).to_vec();
        if tri_indices.len() != 3 {
            continue;
        }

        // check if this triangle is part of the texture, and ignore if not
        let v1 = verts[tri_indices[0]];
        let v2 = verts[tri_indices[1]];
        let v3 = verts[tri_indices[2]];
        let blt = Vec2::new(
            v1.pos.x.min(v2.pos.x).min(v3.pos.x),
            v1.pos.y.min(v2.pos.y).min(v3.pos.y),
        );
        let brb = Vec2::new(
            v1.pos.x.max(v2.pos.x).max(v3.pos.x),
            v1.pos.y.max(v2.pos.y).max(v3.pos.y),
        );
        'pixel_check: for x in (blt.x as i32)..(brb.x as i32) {
            for y in (blt.y as i32)..(brb.y as i32) {
                let bary = tri_point(&Vec2::new(x as f32, y as f32), &v1.pos, &v2.pos, &v3.pos);
                let uv = v1.uv * bary.3 + v2.uv * bary.1 + v3.uv * bary.2;
                let pos = Vec2::new(
                    (uv.x * tex.width() as f32).min(tex.width() as f32 - 1.),
                    (uv.y * tex.height() as f32).min(tex.height() as f32 - 1.),
                );
                let pixel_alpha = tex.get_pixel(pos.x as u32, pos.y as u32).0[3];
                if pixel_alpha > 125 {
                    indices.push(tri_indices[0] as u32);
                    indices.push(tri_indices[1] as u32);
                    indices.push(tri_indices[2] as u32);
                    break 'pixel_check;
                }
            }
        }
    }

    indices
}

fn line_of_sight(img: &DynamicImage, mut p0: Vec2, p1: Vec2) -> bool {
    let dx = (p1.x - p0.x).abs();
    let sx = if p0.x < p1.x { 1 } else { -1 };
    let dy = -(p1.y - p0.y).abs();
    let sy = if p0.y < p1.y { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if p0.x >= 0. && p0.y >= 0. && p0.x < img.width() as f32 && p0.y < img.height() as f32 {
            let px = img.get_pixel(p0.x as u32, p0.y as u32);
            if px[3] == 255 {
                return false;
            }
        }

        if p0.x == p1.x && p0.y == p1.y {
            break;
        }
        let e2 = 2. * err;
        if e2 >= dy {
            err += dy;
            p0.x += sx as f32;
        }
        if e2 <= dx {
            err += dx;
            p0.y += sy as f32;
        }
    }

    true
}
