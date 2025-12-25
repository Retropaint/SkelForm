//! Core rendering logic, abstracted from the rest of WGPU.

use crate::*;
use armature_window::find_bone;
use image::{DynamicImage, GenericImageView};
use spade::Triangulation;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    if shared.window == Vec2::ZERO {
        return;
    }

    shared.ui.scaling = false;
    shared.ui.rotating = false;

    #[cfg(target_arch = "wasm32")]
    loaded();

    // create vert on cursor
    let mut mouse_world_vert = Vertex {
        pos: utils::screen_to_world_space(shared.input.mouse, shared.window),
        ..Default::default()
    };
    mouse_world_vert.pos.x *= shared.window.y / shared.window.x;

    if !shared.config.gridline_front {
        draw_gridline(render_pass, device, shared);
    }

    // create rect textures for all textured bones with no verts
    for b in 0..shared.armature.bones.len() {
        let tex = shared.armature.tex_of(shared.armature.bones[b].id);
        if tex != None && shared.armature.bones[b].vertices.len() == 0 {
            let size = tex.unwrap().size;
            let bone = &mut shared.armature.bones[b];
            (bone.vertices, bone.indices) = create_tex_rect(&size);
            shared.armature.bones[b].verts_edited = false;
        }
    }

    let mut temp_arm = shared.armature.clone();
    let mut anim_bones = shared.animate_bones();

    // adjust anim_bones' verts for new textrues mid-animations
    temp_arm.bones = anim_bones.clone();
    for b in 0..shared.armature.bones.len() {
        let tex = temp_arm.tex_of(shared.armature.bones[b].id);
        if !shared.armature.bones[b].verts_edited && tex != None {
            let size = tex.unwrap().size;
            (anim_bones[b].vertices, anim_bones[b].indices) = create_tex_rect(&size);
        }
    }

    temp_arm.bones = anim_bones.clone();

    // store bound/unbound vert's pos before construction
    let mut init_vert_pos = Vec2::default();
    let vert_id = shared.changed_vert_id as usize;
    if shared.changed_vert_id != -1 {
        init_vert_pos = temp_arm.bones[shared.ui.selected_bone_idx].vertices[vert_id].pos;
    }

    construction(&mut temp_arm.bones, &anim_bones);

    // adjust bound/unbound vert's pos after construction
    if shared.changed_vert_id != -1 {
        let temp_vert = temp_arm.bones[shared.ui.selected_bone_idx].vertices[vert_id];

        let mut diff =
            temp_vert.pos - init_vert_pos - temp_arm.bones[shared.ui.selected_bone_idx].pos;

        // if unbound, vert needs to account for pos in the previous frame
        if let Some(last_frame_pos) = shared.changed_vert_init_pos {
            diff = temp_vert.pos - last_frame_pos;
        }

        let vert = &mut shared.selected_bone_mut().unwrap().vertices[vert_id];
        vert.pos -= diff;
        shared.changed_vert_id = -1;
    }

    let mut mesh_onion_id = -1;

    let mut selected_bones_pos = vec![];
    if shared.selected_bone() != None {
        let id = shared.selected_bone().unwrap().id;
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
        let parents = shared.armature.get_all_parents(temp_arm.bones[b].id);

        let selected_bone = shared.selected_bone();
        if selected_bone != None && selected_bone.unwrap().id == temp_arm.bones[b].id {
            for parent in &parents {
                let tex = temp_arm.tex_of(parent.id);
                if tex != None && parent.verts_edited {
                    mesh_onion_id = parent.id;
                    break;
                }
            }
        }

        if tex == None || temp_arm.bones[b].is_hidden == 1 {
            continue;
        }

        let cam = &shared.world_camera();
        for v in 0..temp_arm.bones[b].vertices.len() {
            let tb = &mut temp_arm.bones[b];
            let vert = world_vert(tb.vertices[v], cam, shared.aspect_ratio(), Vec2::default());
            tb.world_verts.push(vert);
        }

        for vert in &mut temp_arm.bones[b].world_verts {
            vert.add_color = VertexColor::new(0., 0., 0., 0.);
        }
        if shared.ui.setting_bind_verts {
            continue;
        }

        // check if cursor is on an opaque pixel of this bone's texture
        let tb = &temp_arm.bones[b];
        let selected_mesh = !shared.ui.showing_mesh
            || shared.ui.showing_mesh && shared.selected_bone().unwrap().id == tb.id;
        if hover_bone_id == -1 && !shared.input.left_down && !shared.input.on_ui && selected_mesh {
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
                let pos = (v[c0].pos - tb.pos) * bary.3
                    + (v[c1].pos - tb.pos) * bary.1
                    + (v[c2].pos - tb.pos) * bary.2;

                if shared.ui.showing_mesh {
                    if shared.input.right_clicked && !removed_vert {
                        let bone = &mut shared.selected_bone_mut().unwrap();
                        if bone.indices.len() == 6 {
                            shared.ui.open_modal(shared.loc("indices_limit"), false);
                            break;
                        }
                        bone.indices.remove(i * 3);
                        bone.indices.remove(i * 3);
                        bone.indices.remove(i * 3);
                        removed_vert = true;
                        break;
                    }

                    if shared.input.left_clicked && new_vert == None {
                        new_vert = Some(Vertex {
                            pos,
                            uv,
                            ..Default::default()
                        });
                        break;
                    }
                }

                let tex = temp_arm.tex_of(temp_arm.bones[b].id).unwrap();

                let img = &shared.armature.tex_data(tex).unwrap().image;
                let pos = Vec2::new(
                    (uv.x * img.width() as f32).min(img.width() as f32 - 1.),
                    (uv.y * img.height() as f32).min(img.height() as f32 - 1.),
                );
                let pixel_alpha = img.get_pixel(pos.x as u32, pos.y as u32).0[3];
                if pixel_alpha == 255 && !shared.ui.showing_mesh {
                    hover_bone_id = temp_arm.bones[b].id;
                    break;
                }
            }
        }

        let mut click_on_hover_id = temp_arm.bones[b].id;
        if !shared.config.exact_bone_select {
            // QoL: select parent of textured bone if it's called 'Texture'
            // this is because most textured bones are meant to represent their parents
            if parents.len() != 0 && temp_arm.bones[b].name.to_lowercase() == "texture" {
                click_on_hover_id = parents[0].id;
            }
        }

        // hovering glow animation
        if hover_bone_id == temp_arm.bones[b].id && shared.selected_bone_id() != click_on_hover_id {
            let fade = 0.25 * ((shared.time * 3.).sin()).abs() as f32;
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
        if shared.input.left_clicked && hover_bone_id == temp_arm.bones[b].id {
            if shared.ui.setting_ik_target {
                shared.selected_bone_mut().unwrap().ik_target_id = click_on_hover_id;
                shared.ui.setting_ik_target = false;
            } else {
                let idx = &mut shared.ui.selected_bone_idx;
                let bones = &shared.armature.bones;
                let id = click_on_hover_id;
                *idx = bones.iter().position(|bone| bone.id == id).unwrap();
                shared.ui.selected_bone_ids = vec![];

                // unfold all parents that lead to this bone, so it's visible in the hierarchy
                for parent in &parents {
                    shared.armature.find_bone_mut(parent.id).unwrap().folded = false;
                }
            }
        }
    }

    // runtime: sort bones by z-index for drawing
    temp_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    for b in 0..temp_arm.bones.len() {
        let tex = temp_arm.tex_of(temp_arm.bones[b].id);
        if tex == None || temp_arm.bones[b].is_hidden == 1 {
            continue;
        }

        if shared.ui.showing_mesh && shared.selected_bone().unwrap().id == temp_arm.bones[b].id {
            continue;
        }

        let t = tex.unwrap();
        let bg = shared.armature.tex_data(t).unwrap().bind_group.clone();
        let bone = &temp_arm.bones[b];
        draw(&bg, &bone.world_verts, &bone.indices, render_pass, device);
    }

    // draw inverse kinematics arrows

    // todo:
    // only draw arrows for the selected set of bones.
    // currently it shows all when any are selected.
    if shared.selected_bone() != None
        && shared.armature.bone_eff(shared.selected_bone().unwrap().id) != JointEffector::None
    {
        for bone in &temp_arm.bones {
            let bone_eff = shared.armature.bone_eff(bone.id);
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
            let ratio = shared.aspect_ratio();
            let pivot = Vec2::new(0., 0.5);
            for v in 0..4 {
                let verts = arrow.vertices[v];
                let mut new_vert = world_vert(verts, &shared.world_camera(), ratio, pivot);
                new_vert.color = VertexColor::new(1., 1., 1., 0.2);
                arrow.world_verts.push(new_vert);
            }
            let bg = &shared.ik_arrow_bindgroup;
            draw(bg, &arrow.world_verts, &arrow.indices, render_pass, device);
        }
    }

    if shared.ui.showing_mesh || shared.ui.setting_bind_verts {
        let id = shared.selected_bone().unwrap().id;
        let bone = temp_arm.bones.iter().find(|bone| bone.id == id).unwrap();
        let tex = temp_arm.tex_of(bone.id).unwrap();
        let bind_group = &shared.armature.tex_data(tex).unwrap().bind_group;
        let verts = &bone.world_verts;
        draw(&bind_group, &verts, &bone.indices, render_pass, device);
        render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);
        let mouse = mouse_world_vert;
        let nw = &mut new_vert;
        let wv = bone.world_verts.clone();

        let (verts, indices, on_vert) = bone_vertices(&bone.clone(), shared, &wv, true);
        let (lines_v, lines_i, on_line) =
            vert_lines(bone, &temp_arm.bones, shared, &mouse, nw, true, on_vert);

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
            shared.dragging_verts = hovering_tri.iter().map(|v| v.id as usize).collect();
        }
    }

    if mesh_onion_id != -1 {
        render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);
        let tp = &temp_arm.bones;
        let bone = tp.iter().find(|bone| bone.id == mesh_onion_id).unwrap();
        let wv = bone.world_verts.clone();
        let vertex = Vertex::default();

        let (verts, indices, _) = vert_lines(bone, &tp, shared, &vertex, &mut None, true, false);
        draw(&None, &verts, &indices, render_pass, device);

        let (verts, indices, _) = bone_vertices(&bone.clone(), shared, &wv, false);
        draw(&None, &verts, &indices, render_pass, device);
    }

    if !shared.ui.setting_bind_verts {
        if let Some(mut vert) = new_vert {
            shared.new_undo_sel_bone();
            let bone_mut = shared.selected_bone_mut().unwrap();
            let ids = bone_mut.vertices.iter().map(|v| v.id as i32).collect();
            vert.id = generate_id(ids) as u32;
            bone_mut.vertices.push(vert);
            bone_mut.vertices = sort_vertices(bone_mut.vertices.clone());
            bone_mut.indices = triangulate(&bone_mut.vertices);
            bone_mut.verts_edited = true;
        }
    }

    if shared.config.gridline_front {
        draw_gridline(render_pass, device, shared);
    }

    render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);

    let mut color = VertexColor::new(
        shared.config.colors.center_point.r as f32 / 255.,
        shared.config.colors.center_point.g as f32 / 255.,
        shared.config.colors.center_point.b as f32 / 255.,
        0.75,
    );
    let cam = shared.world_camera();
    let zero = Vec2::default();
    let mut point_verts = vec![];
    let mut point_indices = vec![];
    for p in 0..selected_bones_pos.len() {
        if p > 0 {
            color.a = 0.25;
        }
        let pos = selected_bones_pos[p];
        let (mut this_verts, mut this_indices) =
            draw_point(&zero, &shared, &pos, color, cam.pos, 0.);
        for idx in &mut this_indices {
            *idx += p as u32 * 4;
        }
        point_verts.append(&mut this_verts);
        point_indices.append(&mut this_indices);
    }
    if point_indices.len() > 0 {
        draw(&mut None, &point_verts, &point_indices, render_pass, device);
    }

    if !shared.input.left_down {
        shared.dragging_verts = vec![];
        shared.editing_bone = false;
        shared.input.mouse_init = None;
    } else if shared.dragging_verts.len() > 0 {
        let mut bone_id = -1;
        if let Some(bone) = shared.selected_bone() {
            bone_id = bone.id
        }
        for vert in shared.dragging_verts.clone() {
            let bones = &temp_arm.bones;
            let bone = bones.iter().find(|bone| bone.id == bone_id).unwrap();
            drag_vertex(shared, bone, vert);
        }

        return;
    }

    if !shared.input.left_down && !shared.input.right_down {
        return;
    }

    // mouse related stuff

    if shared.input.mouse_init == None {
        shared.input.mouse_init = Some(shared.input.mouse);
        if let Some(bone) = shared.selected_bone() {
            shared.bone_init_rot = bone.rot;
        }
    }

    // move camera
    if (shared.input.holding_mod || shared.input.right_down) && !shared.input.on_ui {
        shared.cursor_icon = egui::CursorIcon::Move;
        shared.camera.pos += shared.mouse_vel() * shared.camera.zoom;
        return;
    }

    let mut ik_disabled = true;
    if let Some(bone) = shared.selected_bone() {
        let is_end = shared.edit_mode == EditMode::Rotate
            && shared.armature.bone_eff(bone.id) == JointEffector::End;
        ik_disabled = is_end
            || (bone.ik_disabled || shared.armature.bone_eff(bone.id) == JointEffector::None);
    }

    if shared.ui.showing_mesh || !ik_disabled {
        return;
    }

    // editing bone
    let sel = shared.ui.selected_bone_idx;
    let input = &shared.input;
    if shared.input.on_ui || shared.ui.polar_modal {
        shared.editing_bone = false;
    } else if sel != usize::MAX && input.left_down && hover_bone_id == -1 && input.down_dur > 5 {
        if shared.edit_mode == EditMode::Rotate {
            let mut mouse = utils::screen_to_world_space(shared.input.mouse, shared.window);
            mouse.x *= shared.aspect_ratio();
            let bone = find_bone(&temp_arm.bones, shared.selected_bone().unwrap().id).unwrap();
            let center = Vertex {
                pos: bone.pos,
                ..Default::default()
            };
            let cam = &shared.world_camera();
            let cw = world_vert(center, cam, shared.aspect_ratio(), Vec2::new(0.5, 0.5));
            draw_line(cw.pos, mouse, shared, render_pass, &device);
        }

        // save bone/animation for undo
        if !shared.editing_bone {
            shared.save_edited_bone();
            //shared.armature.autosave();
            *shared.saving.lock().unwrap() = Saving::Autosaving;
            shared.editing_bone = true;
        }

        shared.cursor_icon = egui::CursorIcon::Crosshair;

        let bone = find_bone(&temp_arm.bones, shared.selected_bone().unwrap().id).unwrap();

        edit_bone(shared, bone, &temp_arm.bones);
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
pub fn render_screenshot(render_pass: &mut RenderPass, device: &Device, shared: &Shared) {
    let mut temp_arm = Armature::default();
    temp_arm.bones = shared.armature.bones.clone();
    construction(&mut temp_arm.bones, &shared.armature.bones);
    temp_arm.bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    let mut cam = shared.world_camera().clone();
    cam.pos = Vec2::new(0., 0.);
    cam.zoom = 1500.;

    for b in 0..temp_arm.bones.len() {
        if shared.armature.tex_of(temp_arm.bones[b].id) == None {
            continue;
        }
        if temp_arm.bones[b].is_hidden == 1 {
            continue;
        }

        for v in 0..temp_arm.bones[b].vertices.len() {
            let tb = &temp_arm.bones[b];
            let mut new_vert =
                world_vert(tb.vertices[v], &cam, shared.aspect_ratio(), Vec2::default());
            new_vert.add_color = VertexColor::new(0., 0., 0., 0.);
            temp_arm.bones[b].world_verts.push(new_vert);
        }

        let arm = &shared.armature;
        let id = temp_arm.bones[b].id;
        let tex = arm.tex_of(id).unwrap();
        let bg = &shared.armature.tex_data(tex).unwrap().bind_group;
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
            vert.offset_rot = bone.rot;
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
                    vert.offset_rot = -bind_bone.rot;
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
                vert.offset_rot = -normal_angle;
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

    match bones[0].ik_mode {
        InverseKinematicsMode::FABRIK => {
            for _ in 0..10 {
                fabrik(bones, root, target);
            }
        }
        InverseKinematicsMode::Arc => arc_ik(bones, root, target),
        _ => {}
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
fn sort_vertices(mut verts: Vec<Vertex>) -> Vec<Vertex> {
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

pub fn edit_bone(shared: &mut Shared, bone: &Bone, bones: &Vec<Bone>) {
    let mut anim_id = shared.ui.anim.selected;
    let anim_frame = shared.ui.anim.selected_frame;
    if !shared.ui.is_animating() {
        anim_id = usize::MAX;
    }

    macro_rules! edit {
        ($bone:expr, $element:expr, $value:expr) => {
            shared.edit_bone($bone.id, &$element, $value, anim_id, anim_frame);
        };
    }

    let vert = Vertex {
        pos: bone.pos,
        ..Default::default()
    };
    let cam = &shared.world_camera();
    let bone_center = world_vert(vert, cam, shared.aspect_ratio(), Vec2::new(0.5, 0.5));

    match shared.edit_mode {
        shared::EditMode::Move => {
            let mut pos = bone.pos;
            let mouse_vel = shared.mouse_vel() * shared.camera.zoom;

            // move position with mouse velocity
            pos -= mouse_vel;

            // restore universal position by offsetting against parents' attributes
            if bone.parent_id != -1 {
                let parent = find_bone(bones, bone.parent_id).unwrap();
                pos -= parent.pos;
                pos = utils::rotate(&pos, -parent.rot);
                pos /= parent.scale;
            }

            edit!(bone, AnimElement::PositionX, pos.x);
            edit!(bone, AnimElement::PositionY, pos.y);
        }
        shared::EditMode::Rotate => {
            shared.ui.rotating = true;

            let mut mouse_init =
                utils::screen_to_world_space(shared.input.mouse_init.unwrap(), shared.window);
            mouse_init.x *= shared.aspect_ratio();
            let dir_init = mouse_init - bone_center.pos;
            let rot_init = dir_init.y.atan2(dir_init.x);

            let mut mouse = utils::screen_to_world_space(shared.input.mouse, shared.window);
            mouse.x *= shared.aspect_ratio();
            let dir = mouse - bone_center.pos;
            let rot = dir.y.atan2(dir.x);

            let rot = shared.bone_init_rot + (rot - rot_init);
            edit!(bone, AnimElement::Rotation, rot);
        }
        shared::EditMode::Scale => {
            shared.ui.scaling = true;

            let mut scale = bone.scale;

            // restore universal scale, by offsetting against parent's
            if bone.parent_id != -1 {
                let parent = find_bone(bones, bone.parent_id).unwrap();
                scale /= parent.scale;
            }

            scale -= shared.mouse_vel();

            edit!(bone, AnimElement::ScaleX, scale.x);
            edit!(bone, AnimElement::ScaleY, scale.y);
        }
    };
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
    bone: &Bone,
    shared: &mut Shared,
    world_verts: &Vec<Vertex>,
    editable: bool,
) -> (Vec<Vertex>, Vec<u32>, bool) {
    let mut all_verts = vec![];
    let mut all_indices = vec![];
    let mut hovering_vert = false;
    let v2z = Vec2::ZERO;
    let rotated = 45. * 3.14 / 180.;
    macro_rules! point {
        ($idx:expr, $color:expr) => {
            draw_point(&world_verts[$idx].pos, &shared, &v2z, $color, v2z, rotated)
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
        let idx = shared.ui.selected_bind as usize;
        let verts: Vec<i32>;
        if idx == usize::MAX {
            verts = vec![];
        } else {
            let selected = shared.selected_bone().unwrap();
            verts = selected.binds[idx].verts.iter().map(|v| v.id).collect();
        }

        let mut col = if idx != usize::MAX && verts.contains(&(world_verts[wv].id as i32)) {
            VertexColor::YELLOW
        } else {
            VertexColor::GREEN
        };
        col.a = if editable { 0.5 } else { 0.15 };
        let (mut verts, mut indices) = point!(wv, col);
        let mouse_on_it = utils::in_bounding_box(&shared.input.mouse, &verts, &shared.window).1;

        if shared.input.on_ui || !mouse_on_it || !editable {
            add_point!(verts, indices, wv);
            continue;
        }

        hovering_vert = true;

        let (mut verts, mut indices) = point!(wv, VertexColor::WHITE);
        add_point!(verts, indices, wv);
        if shared.input.right_clicked {
            if world_verts.len() <= 4 {
                let str_vert_limit = &shared.loc("vert_limit");
                shared.ui.open_modal(str_vert_limit.to_string(), false);
            } else {
                let verts = &mut shared.selected_bone_mut().unwrap().vertices;
                verts.remove(wv);
                *verts = sort_vertices(verts.clone());
                shared.selected_bone_mut().unwrap().indices = triangulate(&verts);

                // remove this vert from its binds
                'bind: for bind in &mut shared.selected_bone_mut().unwrap().binds {
                    for v in 0..bind.verts.len() {
                        if bind.verts[v].id == world_verts[wv].id as i32 {
                            bind.verts.remove(v);
                            break 'bind;
                        }
                    }
                }

                break;
            }
        }
        if !shared.ui.setting_bind_verts {
            if shared.input.left_pressed {
                shared.new_undo_sel_bone();
                shared.dragging_verts = vec![wv];
                break;
            }
        } else if shared.input.left_clicked {
            let idx = shared.ui.selected_bind as usize;
            let vert_id = world_verts[wv].id;
            let bone_mut = &mut shared.selected_bone_mut().unwrap();

            let bind = &bone.binds[idx];
            if let Some(v) = bind.verts.iter().position(|vert| vert.id == vert_id as i32) {
                bone_mut.binds[idx].verts.remove(v);

                let changed_vert_id = world_verts.iter().position(|v| v.id == vert_id).unwrap();
                shared.changed_vert_id = changed_vert_id as i32;

                // store this frame's vert pos for adjustment later
                shared.changed_vert_init_pos = Some(bone.vertices[changed_vert_id].pos);
            } else {
                bone_mut.binds[idx].verts.push(BoneBindVert {
                    id: vert_id as i32,
                    weight: 1.,
                });

                let changed_vert_id = world_verts.iter().position(|v| v.id == vert_id).unwrap();
                shared.changed_vert_init_pos = None;
                shared.changed_vert_id = changed_vert_id as i32;
            }
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
    shared: &mut Shared,
    mouse_world_vert: &Vertex,
    new_vert: &mut Option<Vertex>,
    editable: bool,
    hovering_vert: bool,
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

        let width = 2. * (shared.camera.zoom / 500.);
        let mut base = Vec2::new(width, width) / shared.camera.zoom;
        base = utils::rotate(&base, dir.y.atan2(dir.x));

        let mut col = VertexColor::GREEN;
        col += VertexColor::new(-0.5, -0.5, -0.5, 0.);
        col.a = if editable { 0.3 } else { 0.1 };

        macro_rules! vert {
            ($pos:expr, $v:expr) => {
                Vertex {
                    pos: $pos,
                    color: col,
                    ..$v
                }
            };
        }
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

                if shared.input.left_pressed {
                    shared.dragging_verts.push(i0 as usize);
                    shared.dragging_verts.push(i1 as usize);
                } else if shared.input.left_clicked && !added_vert {
                    let bones = &bones;
                    let v = &bones.iter().find(|b| b.id == bone.id).unwrap().vertices;
                    let wv0 = v[i0 as usize].pos - bone.pos;
                    let wv1 = v[i1 as usize].pos - bone.pos;
                    let pos = wv0 + (wv1 - wv0) * interp;
                    *new_vert = Some(Vertex {
                        pos,
                        uv,
                        ..Default::default()
                    });
                    added_vert = true;
                }
            }

            if is_hovering {
                v0_top.add_color += add_color;
                v0_bot.add_color += add_color;
                v1_top.add_color += add_color;
                v1_bot.add_color += add_color;
            }

            let mv = &shared.dragging_verts;

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

fn draw_line(
    origin: Vec2,
    target: Vec2,
    shared: &Shared,
    render_pass: &mut RenderPass,
    device: &Device,
) {
    let dir = target - origin;

    let width = 2.5;
    let mut base = Vec2::new(width, width) / shared.camera.zoom;
    base = utils::rotate(&base, dir.y.atan2(dir.x));

    let color = VertexColor::new(0., 1., 0., 1.);

    macro_rules! vert {
        ($pos:expr) => {
            Vertex {
                pos: origin + base,
                color,
                ..Default::default()
            }
        };
    }

    let v0_top = vert!(origin + base);
    let v0_bot = vert!(origin - base);
    let v1_top = vert!(target + base);
    let v1_bot = vert!(target - base);

    let verts = vec![v0_top, v0_bot, v1_top, v1_bot];
    let indices = vec![0, 1, 2, 1, 2, 3];

    draw(&None, &verts, &indices, render_pass, device);
}

pub fn drag_vertex(shared: &mut Shared, bone: &Bone, vert_idx: usize) {
    if bone.vertices.len() == 0 || vert_idx > bone.vertices.len() - 1 {
        return;
    }
    let mouse_vel = shared.mouse_vel();
    let zoom = shared.camera.zoom;
    let temp_vert = bone.vertices[vert_idx];
    let og_bone = &mut shared.selected_bone_mut().unwrap();
    og_bone.verts_edited = true;
    let vert_mut = &mut og_bone.vertices[vert_idx];
    vert_mut.pos -= utils::rotate(&(mouse_vel * zoom), temp_vert.offset_rot);
}

pub fn create_tex_rect(tex_size: &Vec2) -> (Vec<Vertex>, Vec<u32>) {
    macro_rules! vert {
        ($pos:expr, $uv:expr, $id:expr) => {
            Vertex {
                pos: $pos,
                uv: $uv,
                id: $id,
                ..Default::default()
            }
        };
    }
    let tex = *tex_size / 2.;
    let mut verts = vec![
        vert!(Vec2::new(-tex.x, tex.y), Vec2::new(0., 0.), 0),
        vert!(Vec2::new(tex.x, tex.y), Vec2::new(1., 0.), 1),
        vert!(Vec2::new(tex.x, -tex.y), Vec2::new(1., 1.), 2),
        vert!(Vec2::new(-tex.x, -tex.y), Vec2::new(0., 1.), 3),
    ];
    verts = sort_vertices(verts.clone());
    let indices = triangulate(&verts);
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

    let mut verts = vec![Vertex {
        pos: Vec2::new(poi[0].x, -poi[0].y),
        uv: Vec2::new(
            poi[0].x / texture.width() as f32,
            poi[0].y / texture.height() as f32,
        ),
        ..Default::default()
    }];
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

    bone_panel::center_verts(&mut verts);

    (verts.clone(), triangulate(&verts))
}

fn draw_point(
    offset: &Vec2,
    shared: &Shared,
    pos: &Vec2,
    color: VertexColor,
    camera: Vec2,
    rotation: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let point_size = 6. * (shared.camera.zoom / 500.);
    macro_rules! vert {
        ($pos:expr, $uv:expr) => {
            Vertex {
                pos: $pos,
                uv: $uv,
                color,
                ..Default::default()
            }
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
    let ar = shared.aspect_ratio();
    let mut cam = shared.world_camera().clone();
    cam.pos = camera;
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
        mag_filter: wgpu::FilterMode::Linear,
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

fn draw_gridline(render_pass: &mut RenderPass, device: &Device, shared: &Shared) {
    render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);

    let cam = shared.world_camera();

    let col = VertexColor::new(
        shared.config.colors.gridline.r as f32 / 255.,
        shared.config.colors.gridline.g as f32 / 255.,
        shared.config.colors.gridline.b as f32 / 255.,
        1.,
    );

    let width = 0.005 * cam.zoom;
    let regular_color = VertexColor::new(col.r, col.g, col.b, 0.15);
    let highlight_color = VertexColor::new(col.r, col.g, col.b, 1.);

    let mut verts = vec![];
    let mut indices: Vec<u32> = vec![];
    let mut i: u32 = 0;

    // draw vertical lines
    let mut x = (cam.pos.x - cam.zoom / shared.aspect_ratio()).round();
    let right_side = cam.pos.x + cam.zoom / shared.aspect_ratio();
    while x < right_side {
        if x % shared.gridline_gap as f32 != 0. {
            x += 1.;
            continue;
        }
        let color = if x == 0. {
            highlight_color
        } else {
            regular_color
        };
        verts.append(&mut draw_vertical_line(x, width, shared, color));
        indices.append(&mut vec![i, i + 1, i + 2]);
        i += 3;
        x += 1.;
    }

    // draw horizontal lines
    let mut y = (cam.pos.y - cam.zoom).round();
    let top_side = cam.pos.y + cam.zoom;
    while y < top_side {
        if y % shared.gridline_gap as f32 != 0. {
            y += 1.;
            continue;
        }
        let color = if y == 0. {
            highlight_color
        } else {
            regular_color
        };
        verts.append(&mut draw_horizontal_line(y, width, shared, color));
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
    shared: &Shared,
    color: VertexColor,
) -> Vec<Vertex> {
    let edge = shared.camera.zoom * 5.;
    let c = &shared.world_camera();
    let vertices: Vec<Vertex> = vec![
        vert!((Vec2::new(c.pos.x - edge, y) - c.pos) / c.zoom, color),
        vert!((Vec2::new(c.pos.x, width + y) - c.pos) / c.zoom, color),
        vert!((Vec2::new(c.pos.x + edge, y) - c.pos) / c.zoom, color),
    ];
    vertices
}

pub fn draw_vertical_line(x: f32, width: f32, shared: &Shared, color: VertexColor) -> Vec<Vertex> {
    let edge = shared.camera.zoom * 5.;
    let c = &shared.world_camera();
    let r = shared.aspect_ratio();
    let vertices: Vec<Vertex> = vec![
        vert!((Vec2::new(x, c.pos.y - edge) - c.pos) / c.zoom * r, color),
        vert!((Vec2::new(width + x, c.pos.y) - c.pos) / c.zoom * r, color),
        vert!((Vec2::new(x, c.pos.y + edge) - c.pos) / c.zoom * r, color),
    ];
    vertices
}

pub fn triangulate(verts: &Vec<Vertex>) -> Vec<u32> {
    let mut triangulation: spade::DelaunayTriangulation<_> = spade::DelaunayTriangulation::new();

    for vert in verts {
        let _ = triangulation.insert(spade::Point2::new(vert.uv.x, vert.uv.y));
    }

    let mut indices: Vec<u32> = Vec::new();
    for face in triangulation.inner_faces() {
        let tri_indices = face.vertices().map(|v| v.index()).to_vec();

        if tri_indices.len() == 3 {
            indices.push(tri_indices[0] as u32);
            indices.push(tri_indices[1] as u32);
            indices.push(tri_indices[2] as u32);
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
