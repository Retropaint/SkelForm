//! Core rendering logic, abstracted from the rest of WGPU.

use crate::*;
use armature_window::find_bone;
use image::DynamicImage;
use image::GenericImageView;
use spade::Triangulation;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};
use winit::keyboard::KeyCode;

macro_rules! con_vert {
    ($func:expr, $vert:expr, $bone:expr, $tex_size:expr, $cam_pos:expr, $cam_zoom:expr) => {
        $func($vert, Some(&$bone), &$cam_pos, $cam_zoom, $tex_size, 1., 1.)
    };
}

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
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

    for bone in &mut shared.armature.bones {
        if bone.tex_set_idx == -1 || bone.vertices.len() != 0 {
            continue;
        }

        let tex_size = shared.armature.texture_sets[bone.tex_set_idx as usize].textures
            [bone.tex_idx as usize]
            .size;
        (bone.vertices, bone.indices) = create_tex_rect(&tex_size);
    }

    let mut bones = shared.armature.bones.clone();
    if shared.ui.anim.open {
        let mut playing = false;
        for a in 0..shared.armature.animations.len() {
            let anim = &mut shared.armature.animations[a];
            if anim.elapsed == None {
                continue;
            }
            playing = true;
            let frame = anim.set_frame();
            bones = shared.armature.animate(a, frame, Some(&bones));
        }
        if !playing && shared.ui.anim.selected_frame != -1 {
            bones = shared.armature.animate(
                shared.ui.anim.selected,
                shared.ui.anim.selected_frame,
                None,
            );
        }
    }

    // armature bones should be immutable to rendering
    let mut temp_bones: Vec<Bone> = bones.clone();

    let mut ik_rot: std::collections::HashMap<i32, f32> = std::collections::HashMap::new();

    // runtime: constructing rig using forward (aka inheritance) & inverse kinematics
    {
        forward_kinematics(&mut temp_bones, std::collections::HashMap::new());

        // inverse kinematics
        for b in 0..temp_bones.len() {
            if temp_bones[b].joint_effector != JointEffector::Start || temp_bones[b].ik_disabled {
                continue;
            }

            let mouse_world = utils::screen_to_world_space(shared.input.mouse, shared.window);
            if mouse_world.x.is_nan() {
                break;
            }

            // get all joint children of this bone, including itself
            let mut joints = vec![];
            armature_window::get_all_children(&temp_bones, &mut joints, &temp_bones[b]);
            joints.insert(0, temp_bones[b].clone());
            joints = joints
                .iter()
                .filter(|joint| joint.joint_effector != JointEffector::None)
                .cloned()
                .collect();

            // apply IK on the joint copy, then save rotations for the next FK
            let target = temp_bones
                .iter()
                .find(|bone| bone.id == temp_bones[b].ik_target_id);

            if target == None {
                continue;
            }

            for _ in 0..10 {
                inverse_kinematics(&mut joints, target.unwrap().pos);
            }
            for joint in joints {
                ik_rot.insert(joint.id, joint.rot);
            }
        }

        // re-construct bones, accounting for IK
        temp_bones = bones.clone();
        forward_kinematics(&mut temp_bones, ik_rot.clone());
    }

    // sort bones by highest zindex first, so that hover logic will pick the top-most one
    temp_bones.sort_by(|a, b| b.zindex.total_cmp(&a.zindex));

    let mut hover_bone_id = -1;

    // many fight for spot of newest vertex; there will only be one.
    let mut new_vert: Option<Vertex> = None;
    let mut added_verts = false;

    // pre-draw bone setup
    for b in 0..temp_bones.len() {
        if temp_bones[b].tex_set_idx == -1 {
            continue;
        }

        let set = &shared.armature.texture_sets[temp_bones[b].tex_set_idx as usize];
        let out_of_bounds = temp_bones[b].tex_idx > set.textures.len() as i32 - 1;
        if shared.armature.is_bone_hidden(temp_bones[b].id) || out_of_bounds {
            continue;
        }

        let tex_size = shared.armature.texture_sets[temp_bones[b].tex_set_idx as usize].textures
            [temp_bones[b].tex_idx as usize]
            .size;
        for v in 0..temp_bones[b].vertices.len() {
            let mut new_vert = con_vert!(
                raw_to_world_vert,
                temp_bones[b].vertices[v],
                temp_bones[b],
                tex_size,
                shared.camera.pos,
                shared.camera.zoom
            );
            new_vert.pos.x /= shared.window.x / shared.window.y;
            temp_bones[b].world_verts.push(new_vert);
        }

        // check if cursor is on an opaque pixel of this bone's texture
        if hover_bone_id == -1 && !shared.input.left_down && !shared.input.on_ui {
            let tb = temp_bones[b].clone();
            for (_, chunk) in tb.indices.chunks_exact(3).enumerate() {
                let bary = tri_point(
                    &mouse_world_vert.pos,
                    &tb.world_verts[chunk[0] as usize].pos,
                    &tb.world_verts[chunk[1] as usize].pos,
                    &tb.world_verts[chunk[2] as usize].pos,
                );

                if bary.0 == -1. {
                    continue;
                }

                let uv = temp_bones[b].world_verts[chunk[0] as usize].uv * bary.3
                    + temp_bones[b].world_verts[chunk[1] as usize].uv * bary.1
                    + temp_bones[b].world_verts[chunk[2] as usize].uv * bary.2;

                let pixel_pos: Vec2;

                let img = &shared.armature.texture_sets[temp_bones[b].tex_set_idx as usize]
                    .textures[temp_bones[b].tex_idx as usize]
                    .image;
                pixel_pos = Vec2::new(
                    (uv.x * img.width() as f32).min(img.width() as f32 - 1.),
                    (uv.y * img.height() as f32).min(img.height() as f32 - 1.),
                );

                if shared.input.left_clicked && shared.ui.editing_mesh && !added_verts {
                    new_vert = Some(Vertex {
                        pos: Vec2::new(pixel_pos.x, -pixel_pos.y),
                        uv,
                        ..Default::default()
                    });
                    added_verts = true;
                }

                let pixel_alpha = shared.armature.texture_sets[temp_bones[b].tex_set_idx as usize]
                    .textures[temp_bones[b].tex_idx as usize]
                    .image
                    .get_pixel(pixel_pos.x as u32, pixel_pos.y as u32)
                    .0[3];
                if pixel_alpha == 255 && !shared.ui.editing_mesh {
                    hover_bone_id = temp_bones[b].id;
                    break;
                }
            }
        }

        let mut click_on_hover_id = temp_bones[b].id;
        if !shared.config.exact_bone_select {
            // QoL: select first untextured parent of this bone
            // this is because most textured bones are meant to represent their parents
            let parents = shared.armature.get_all_parents(temp_bones[b].id);
            for parent in &parents {
                if parent.tex_set_idx == -1 {
                    click_on_hover_id = parent.id;
                    break;
                }
            }
        }

        // hovering glow animation
        if hover_bone_id == temp_bones[b].id && shared.selected_bone_id() != click_on_hover_id {
            let fade = 0.25 * ((shared.time * 3.).sin()).abs() as f32;
            let min = 0.1;
            for vert in &mut temp_bones[b].world_verts {
                vert.add_color = VertexColor::new(min + fade, min + fade, min + fade, 0.);
            }
        } else {
            for vert in &mut temp_bones[b].world_verts {
                vert.add_color = VertexColor::new(0., 0., 0., 0.);
            }
        }

        // select bone on click
        if shared.input.left_clicked && hover_bone_id == temp_bones[b].id {
            if shared.ui.setting_ik_target {
                shared.selected_bone_mut().unwrap().ik_target_id = click_on_hover_id;
                shared.ui.setting_ik_target = false;
            } else {
                shared.ui.selected_bone_idx = shared
                    .armature
                    .bones
                    .iter()
                    .position(|bone| bone.id == click_on_hover_id)
                    .unwrap();

                // unfold all parents that lead to this bone, so it's visible in the hierarchy
                let parents = shared.armature.get_all_parents(click_on_hover_id);
                for parent in &parents {
                    shared.armature.find_bone_mut(parent.id).unwrap().folded = false;
                }
            }
        }
    }

    // runtime: sort bones by z-index for drawing
    temp_bones.sort_by(|a, b| a.zindex.total_cmp(&b.zindex));

    for bone in &mut temp_bones {
        if bone.tex_set_idx == -1 || shared.armature.is_bone_hidden(bone.id) {
            continue;
        }

        if shared.ui.editing_mesh && shared.selected_bone().unwrap().id == bone.id {
            continue;
        }

        let bind_group = &shared.armature.texture_sets[bone.tex_set_idx as usize].textures
            [bone.tex_idx as usize]
            .bind_group;
        draw(
            &bind_group,
            &bone.world_verts,
            &bone.indices,
            render_pass,
            device,
        );
    }

    if shared.selected_bone() != None
        && shared.selected_bone().unwrap().joint_effector != JointEffector::None
    {
        for bone in &temp_bones {
            if bone.joint_effector == JointEffector::None
                || bone.joint_effector == JointEffector::End
            {
                continue;
            }
            let mut arrow = Bone {
                pos: bone.pos,
                rot: bone.rot,
                scale: Vec2::new(2., 2.),
                pivot: Vec2::new(0., 0.5),
                ..Default::default()
            };
            let tex_size = Vec2::new(61., 48.);
            (arrow.vertices, arrow.indices) = create_tex_rect(&tex_size);
            for v in 0..4 {
                let mut new_vert = con_vert!(
                    raw_to_world_vert,
                    arrow.vertices[v],
                    arrow,
                    tex_size,
                    shared.camera.pos,
                    shared.camera.zoom
                );
                new_vert.pos.x /= shared.window.x / shared.window.y;
                new_vert.color = VertexColor::new(1., 1., 1., 0.2);
                arrow.world_verts.push(new_vert);
            }
            draw(
                &shared.ik_arrow_bindgroup,
                &arrow.world_verts,
                &arrow.indices,
                render_pass,
                device,
            );
        }
    }

    if shared.ui.editing_mesh {
        let bone = temp_bones
            .iter_mut()
            .find(|bone| bone.id == shared.selected_bone().unwrap().id)
            .unwrap();
        let bind_group = &shared.armature.texture_sets[bone.tex_set_idx as usize].textures
            [bone.tex_idx as usize]
            .bind_group;
        draw(
            &bind_group,
            &bone.world_verts,
            &bone.indices,
            render_pass,
            device,
        );

        render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);

        vert_lines(
            &bone,
            shared,
            &mouse_world_vert,
            render_pass,
            device,
            &mut new_vert,
        );
        bone_vertices(&bone, shared, render_pass, device, &bone.world_verts);
    }

    if let Some(vert) = new_vert {
        shared.undo_actions.push(Action {
            action: ActionEnum::Bone,
            id: shared.selected_bone().unwrap().id,
            bones: vec![shared.selected_bone().unwrap().clone()],
            ..Default::default()
        });
        let bone_mut = shared.selected_bone_mut().unwrap();
        bone_mut.vertices.push(vert);
        bone_mut.vertices = sort_vertices(bone_mut.vertices.clone());
        bone_mut.indices = triangulate(&bone_mut.vertices);
    }

    if shared.config.gridline_front {
        draw_gridline(render_pass, device, shared);
    }

    render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);

    if shared.selected_bone() != None {
        let color = VertexColor::new(
            shared.config.colors.center_point.r as f32 / 255.,
            shared.config.colors.center_point.g as f32 / 255.,
            shared.config.colors.center_point.b as f32 / 255.,
            0.5,
        );
        draw_point(
            &Vec2::ZERO,
            &shared,
            render_pass,
            device,
            &find_bone(&temp_bones, shared.selected_bone().unwrap().id).unwrap(),
            color,
            shared.camera.pos,
            0.,
        );
    }

    if !shared.input.left_down {
        shared.dragging_verts = vec![];
        shared.editing_bone = false;
        return;
    } else if shared.dragging_verts.len() > 0 {
        let mut bone_id = -1;
        if let Some(bone) = shared.selected_bone() {
            bone_id = bone.id
        }
        for vert in shared.dragging_verts.clone() {
            drag_vertex(
                shared,
                &temp_bones.iter().find(|bone| bone.id == bone_id).unwrap(),
                vert,
            );
        }

        return;
    }

    // mouse related stuff

    // move camera
    if (shared.input.is_pressing(KeyCode::SuperLeft)
        || shared.input.right_down
        || shared.ui.selected_bone_idx == usize::MAX)
        && !shared.input.on_ui
    {
        shared.camera.pos += shared.mouse_vel() * shared.camera.zoom;
        return;
    }

    let mut ik_disabled = true;
    if let Some(bone) = shared.selected_bone() {
        ik_disabled = bone.ik_disabled || bone.joint_effector == JointEffector::None;
    }

    if shared.ui.editing_mesh || !ik_disabled {
        return;
    }

    // editing bone
    if shared.input.on_ui || shared.ui.has_state(UiState::PolarModal) {
        shared.editing_bone = false;
    } else if shared.ui.selected_bone_idx != usize::MAX && shared.input.left_down {
        // save bone/animation for undo
        if !shared.editing_bone {
            shared.save_edited_bone();
            //shared.armature.autosave();
            shared.saving = Saving::Autosaving;
            shared.editing_bone = true;
        }

        shared.cursor_icon = egui::CursorIcon::Crosshair;

        let bone = find_bone(&temp_bones, shared.selected_bone().unwrap().id).unwrap();
        edit_bone(shared, bone, &temp_bones);
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
        return (
            area,
            s_normalized,
            t_normalized,
            1. - (s_normalized + t_normalized),
        );
    }

    (-1., -1., -1., -1.)
}

/// Stripped-down renderer for screenshot purposes.
pub fn render_screenshot(render_pass: &mut RenderPass, device: &Device, shared: &Shared) {
    let mut temp_bones: Vec<Bone> = shared.armature.bones.clone();
    forward_kinematics(&mut temp_bones, std::collections::HashMap::new());
    temp_bones.sort_by(|a, b| a.zindex.total_cmp(&b.zindex));

    let zoom = 1000.;

    for b in 0..temp_bones.len() {
        if temp_bones[b].tex_set_idx == -1 {
            continue;
        }
        let set = &shared.armature.texture_sets[temp_bones[b].tex_set_idx as usize];
        if shared.armature.is_bone_hidden(temp_bones[b].id)
            || temp_bones[b].tex_idx > set.textures.len() as i32 - 1
        {
            continue;
        }

        let tex_size = set.textures[temp_bones[b].tex_idx as usize].size;
        for v in 0..temp_bones[b].vertices.len() {
            let mut new_vert = con_vert!(
                raw_to_world_vert,
                temp_bones[b].vertices[v],
                temp_bones[b],
                tex_size,
                Vec2::new(0., 0.),
                zoom
            );
            new_vert.pos.x /= shared.window.x / shared.window.y;
            new_vert.add_color = VertexColor::new(0., 0., 0., 0.);
            temp_bones[b].world_verts.push(new_vert);
        }

        let bind_group = &shared.armature.texture_sets[temp_bones[b].tex_set_idx as usize].textures
            [temp_bones[b].tex_idx as usize]
            .bind_group;
        draw(
            bind_group,
            &temp_bones[b].world_verts,
            &temp_bones[b].indices,
            render_pass,
            device,
        );
    }
}

// https://www.youtube.com/watch?v=NfuO66wsuRg
pub fn inverse_kinematics(bones: &mut Vec<Bone>, target: Vec2) {
    let root = bones
        .iter_mut()
        .find(|bone| bone.joint_effector == JointEffector::Start)
        .unwrap()
        .pos;

    // forward-reaching
    let mut next_pos: Vec2 = target;
    let mut next_length = 0.;
    for b in (0..bones.len()).rev() {
        let mut length = (next_pos - bones[b].pos).normalize() * next_length;
        if length.x.is_nan() {
            length = Vec2::new(0., 0.);
        }
        if b == 0 {
            next_length = 0.;
        } else {
            next_length = (bones[b].pos - bones[b - 1].pos).mag();
        }
        bones[b].pos = next_pos - length;

        // get local angle of joint
        let joint_dir = (next_pos - bones[b].pos).normalize();
        let base_dir = (target - root).normalize();
        let joint_angle = joint_dir.y.atan2(joint_dir.x) - base_dir.y.atan2(base_dir.x);

        let const_min;
        let const_max;
        match bones[b].constraint {
            JointConstraint::None => {
                const_min = -3.14;
                const_max = 3.14;
            }
            JointConstraint::Clockwise => {
                const_min = -3.14;
                const_max = 0.;
            }
            JointConstraint::CounterClockwise => {
                const_min = 0.;
                const_max = 3.14;
            }
        }

        // if joint angle is beyond constraint, rotate the hinge so it's on the opposite side
        if (joint_angle > const_max || joint_angle < const_min) && b < bones.len() - 1 {
            let rot_offset = -joint_angle * 2.;
            let rotated = utils::rotate(&(bones[b].pos - bones[b + 1].pos), rot_offset);
            bones[b].pos = rotated + bones[b + 1].pos;
        }

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
        if b == bones.len() - 1 {
            prev_length = 0.;
        } else {
            prev_length = (bones[b].pos - bones[b + 1].pos).mag();
        }

        bones[b].pos = prev_pos - length;
        prev_pos = bones[b].pos;
    }

    // rotating bones
    let end_bone = bones
        .iter_mut()
        .find(|bone| bone.joint_effector == JointEffector::End);
    if end_bone == None {
        return;
    }
    let mut tip_pos = end_bone.unwrap().pos;
    for b in (0..bones.len()).rev() {
        let eff = bones[b].joint_effector.clone();
        if eff == JointEffector::None || eff == JointEffector::End {
            continue;
        }

        let dir = tip_pos - bones[b].pos;
        bones[b].rot = dir.y.atan2(dir.x);
        tip_pos = bones[b].pos;
    }
}

pub fn ik_bone(bone: &Bone, target: Vec2, end: Vec2) -> f32 {
    if bone.joint_effector == JointEffector::None {
        return bone.rot;
    }

    let ei = end - bone.pos;
    let ti = target - bone.pos;

    let angle = ei.y.atan2(ei.x) - ti.y.atan2(ti.x);
    angle
}

/// sort vertices in cw (or ccw?) order
fn sort_vertices(mut verts: Vec<Vertex>) -> Vec<Vertex> {
    // get center point
    let mut center = Vec2::default();
    for v in &verts {
        center += v.pos;
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
    if !shared.ui.is_animating() {
        anim_id = usize::MAX;
    }

    macro_rules! edit {
        ($element:expr, $value:expr) => {
            shared.armature.edit_bone(
                shared.selected_bone().unwrap().id,
                &$element,
                $value,
                anim_id,
                shared.ui.anim.selected_frame,
            );
        };
    }

    match shared.edit_mode {
        shared::EditMode::Move => {
            let mut pos = bone.pos;

            // move position with mouse velocity
            pos -= shared.mouse_vel() * shared.camera.zoom;

            // restore universal position, by offsetting against parents' attributes
            if bone.parent_id != -1 {
                let parent = find_bone(bones, bone.parent_id).unwrap();
                pos -= parent.pos;
                pos = utils::rotate(&pos, -parent.rot);
                pos /= parent.scale;
            }

            edit!(AnimElement::PositionX, pos.x);
            edit!(AnimElement::PositionY, pos.y);
        }
        shared::EditMode::Rotate => {
            let rot = (shared.input.mouse.x / shared.window.x) * std::f32::consts::PI * 2.;
            edit!(AnimElement::Rotation, rot);
        }
        shared::EditMode::Scale => {
            let scale = (shared.input.mouse / shared.window) * 2.;
            edit!(AnimElement::ScaleX, scale.x);
            edit!(AnimElement::ScaleY, scale.y);
        }
    };
}

pub fn forward_kinematics(bones: &mut Vec<Bone>, init_rot: std::collections::HashMap<i32, f32>) {
    for i in 0..bones.len() {
        let mut parent: Option<Bone> = None;
        for b in 0..bones.len() {
            if bones[b].id == bones[i].parent_id {
                parent = Some(bones[b].clone());
                break;
            }
        }

        let id = bones[i].id;
        if init_rot.get(&id) != None {
            if parent == None {
                bones[i].rot = *init_rot.get(&id).unwrap();
            } else {
                bones[i].rot = *init_rot.get(&id).unwrap() - parent.as_ref().unwrap().rot;
            }
        }

        if parent != None {
            inherit_from_parent(&mut bones[i], parent.as_ref().unwrap());
        }
    }
}

pub fn inherit_from_parent(child: &mut Bone, parent: &Bone) {
    child.rot += parent.rot;
    child.scale *= parent.scale;

    // adjust bone's position based on parent's scale
    child.pos *= parent.scale;

    // rotate such that it will orbit the parent
    child.pos = utils::rotate(&child.pos, parent.rot);

    // inherit position from parent
    child.pos += parent.pos;
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
    render_pass: &mut RenderPass,
    device: &Device,
    world_verts: &Vec<Vertex>,
) {
    macro_rules! point {
        ($idx:expr, $color:expr) => {
            draw_point(
                &world_verts[$idx].pos,
                &shared,
                render_pass,
                device,
                &Bone {
                    pos: Vec2::ZERO,
                    ..bone.clone()
                },
                $color,
                Vec2::ZERO,
                45. * 3.14 / 180.,
            )
        };
    }

    for wv in 0..world_verts.len() {
        let point = point!(wv, VertexColor::GREEN);
        let mouse_on_it = utils::in_bounding_box(&shared.input.mouse, &point, &shared.window).1;

        if shared.input.on_ui || !mouse_on_it {
            continue;
        }

        point!(wv, VertexColor::WHITE);
        if shared.input.right_clicked {
            if world_verts.len() <= 4 {
                let str_vert_limit = shared.loc("vert_limit");
                shared.ui.open_modal(str_vert_limit.to_string(), false);
            } else {
                let verts = &mut shared.selected_bone_mut().unwrap().vertices;
                verts.remove(wv);
                *verts = sort_vertices(verts.clone());
                shared.selected_bone_mut().unwrap().indices = triangulate(&verts);
                break;
            }
        }
        if shared.input.left_pressed {
            shared.undo_actions.push(Action {
                action: ActionEnum::Bone,
                bones: vec![shared.selected_bone().unwrap().clone()],
                id: shared.selected_bone().unwrap().id,
                ..Default::default()
            });
            shared.dragging_verts = vec![wv];
            break;
        }
    }
}

pub fn vert_lines(
    bone: &Bone,
    shared: &mut Shared,
    mouse_world_vert: &Vertex,
    render_pass: &mut RenderPass,
    device: &Device,
    new_vert: &mut Option<Vertex>,
) {
    let mut clicked = false;
    let mut lines: Vec<(u32, u32)> = vec![];
    for (_, chunk) in bone.indices.chunks_exact(3).enumerate() {
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
        let width = 2.5;
        let mut size = Vec2::new(width, width) / shared.camera.zoom;
        size = utils::rotate(&size, dir.y.atan2(dir.x));
        let mut col = VertexColor::GREEN;
        col += VertexColor::new(-0.5, -0.5, -0.5, 0.);

        let mut v0_top = Vertex {
            pos: v0.pos + size,
            color: col,
            ..v0
        };
        let mut v0_bot = Vertex {
            pos: v0.pos - size,
            color: col,
            ..v0
        };
        let mut v1_top = Vertex {
            pos: v1.pos + size,
            color: col,
            ..v1
        };
        let mut v1_bot = Vertex {
            pos: v1.pos - size,
            color: col,
            ..v1
        };

        let mut verts = vec![v0_top, v0_bot, v1_top, v1_bot];
        let indices = vec![0, 1, 2, 1, 2, 3];
        let add_color = VertexColor::new(0.2, 0.2, 0.2, 0.);

        let mut is_hovering = false;

        for (_, chunk) in indices.chunks_exact(3).enumerate() {
            let bary = tri_point(
                &mouse_world_vert.pos,
                &verts[chunk[0] as usize].pos,
                &verts[chunk[1] as usize].pos,
                &verts[chunk[2] as usize].pos,
            );
            if bary.0 == -1. {
                continue;
            }
            is_hovering = true;

            let mouse_line = mouse_world_vert.pos - v0.pos;
            let whole_line = v1.pos - v0.pos;
            let interp = mouse_line.mag() / whole_line.mag();
            let uv = v0.uv + ((v1.uv - v0.uv) * interp);
            if shared.input.left_clicked && !clicked {
                let img = &shared.armature.texture_sets[bone.tex_set_idx as usize].textures
                    [bone.tex_idx as usize]
                    .image;
                let pos = Vec2::new(
                    (uv.x * img.width() as f32).min(img.width() as f32 - 1.),
                    -(uv.y * img.height() as f32).min(img.height() as f32 - 1.),
                );
                *new_vert = Some(Vertex {
                    pos,
                    uv,
                    ..Default::default()
                });
                clicked = true;
            } else if shared.input.left_pressed {
                shared.dragging_verts.push(i0 as usize);
                shared.dragging_verts.push(i1 as usize);
            }
        }

        if is_hovering {
            v0_top.add_color += add_color;
            v0_bot.add_color += add_color;
            v1_top.add_color += add_color;
            v1_bot.add_color += add_color;
            verts = vec![v0_top, v0_bot, v1_top, v1_bot];
        }

        if shared.dragging_verts.len() == 2
            && shared.dragging_verts[0] == i0 as usize
            && shared.dragging_verts[1] == i1 as usize
        {
            v0_top.add_color += add_color;
            v0_bot.add_color += add_color;
            v1_top.add_color += add_color;
            v1_bot.add_color += add_color;
            verts = vec![v0_top, v0_bot, v1_top, v1_bot];
        }

        draw(&None, &verts, &indices, render_pass, device);
    }
}

pub fn drag_vertex(shared: &mut Shared, bone: &Bone, vert_idx: usize) {
    let tex_size = shared.armature.texture_sets[bone.tex_set_idx as usize].textures
        [bone.tex_idx as usize]
        .size;
    // when moving a vertex, it must be interpreted in world coords first to align the with the mouse
    let mut world_vert = con_vert!(
        raw_to_world_vert,
        bone.vertices[vert_idx],
        bone,
        tex_size,
        shared.camera.pos,
        shared.camera.zoom
    );

    // now that it's in world coords, it can follow the mouse
    world_vert.pos -= shared.mouse_vel();

    // convert back to normal coords
    let vert_pos = con_vert!(
        world_to_raw_vert,
        world_vert,
        bone,
        tex_size,
        shared.camera.pos,
        shared.camera.zoom
    )
    .pos;

    if !shared.ui.is_animating() {
        shared.selected_bone_mut().unwrap().vertices[vert_idx].pos = vert_pos;
        return;
    }

    let og_vert_pos = bone.vertices[vert_idx].pos;
    let final_pos = vert_pos - og_vert_pos;

    shared.armature.edit_vert(
        bone.id,
        vert_idx as i32,
        &(final_pos),
        shared.ui.anim.selected,
        shared.ui.anim.selected_frame,
    );
}

pub fn create_tex_rect(tex_size: &Vec2) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = vec![
        Vertex::default(),
        Vertex {
            pos: Vec2::new(tex_size.x, 0.),
            uv: Vec2::new(1., 0.),
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(tex_size.x, -tex_size.y),
            uv: Vec2::new(1., 1.),
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(0., -tex_size.y),
            uv: Vec2::new(0., 1.),
            ..Default::default()
        },
    ];
    verts = sort_vertices(verts.clone());
    let indices = triangulate(&verts);
    (verts, indices)
}

pub fn polygonate(texture: &image::DynamicImage) -> (Vec<Vertex>, Vec<u32>) {
    let gap = 25.;
    let mut poi: Vec<Vec2> = vec![];

    // create spaced-out points of interest
    let mut cursor = Vec2::default();
    while cursor.y < texture.height() as f32 {
        if texture.get_pixel(cursor.x as u32, cursor.y as u32).0[3] == 0 {
            poi.push(cursor);
        }
        cursor.x += gap;
        if cursor.x > texture.width() as f32 {
            cursor.x = 0.;
            cursor.y += gap;
        }
    }

    // only keep points that are close to the image
    // redundant points are determined if all 8 neighbouring coords have points (or is in bounds)
    let poi_clone = poi.clone();
    poi.retain(|point| {
        let left = Vec2::new(point.x - gap, point.y);
        let right = Vec2::new(point.x + gap, point.y);
        let up = Vec2::new(point.x, point.y + gap);
        let down = Vec2::new(point.x, point.y - gap);

        let left_top = Vec2::new(point.x - gap, point.y + gap);
        let left_bot = Vec2::new(point.x - gap, point.y - gap);
        let right_top = Vec2::new(point.x + gap, point.y + gap);
        let right_bot = Vec2::new(point.x + gap, point.y - gap);

        macro_rules! p {
            ($dir:expr) => {
                !poi_clone.contains($dir)
                    && $dir.x > 0.
                    && $dir.y > 0.
                    && $dir.x < texture.width() as f32
                    && $dir.y < texture.height() as f32
            };
        }

        p!(&left)
            || p!(&right)
            || p!(&up)
            || p!(&down)
            || p!(&left_top)
            || p!(&left_bot)
            || p!(&right_top)
            || p!(&right_bot)
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

    // get last point that current one has line of sight one
    // if next point checked happens to be first and there's line of sight, tracing is over
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

        verts.push(Vertex {
            pos: Vec2::new(poi[p - 1].x, -poi[p - 1].y),
            uv: Vec2::new(
                poi[p - 1].x / texture.width() as f32,
                poi[p - 1].y / texture.height() as f32,
            ),
            ..Default::default()
        });
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

    (verts.clone(), triangulate(&verts))
}

fn draw_point(
    offset: &Vec2,
    shared: &Shared,
    render_pass: &mut RenderPass,
    device: &Device,
    bone: &Bone,
    color: VertexColor,
    camera: Vec2,
    rotation: f32,
) -> Vec<Vertex> {
    let point_size = 10.;
    let mut temp_point_verts: [Vertex; 4] = [
        Vertex {
            pos: Vec2::new(-point_size, point_size),
            uv: Vec2::new(1., 0.),
            color,
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(point_size, point_size),
            uv: Vec2::new(0., 1.),
            color,
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(-point_size, -point_size),
            uv: Vec2::new(0., 0.),
            color,
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(point_size, -point_size),
            uv: Vec2::new(1., 1.),
            color,
            ..Default::default()
        },
    ];

    for v in &mut temp_point_verts {
        v.pos += bone.pos;
        v.pos = utils::rotate(&v.pos, rotation);
    }

    let mut point_verts = vec![];
    for i in 0..temp_point_verts.len() {
        point_verts.push(raw_to_world_vert(
            temp_point_verts[i],
            None,
            &camera,
            shared.camera.zoom,
            Vec2::new(0., 0.),
            shared.window.x / shared.window.y,
            1.,
        ));
    }

    for vert in &mut point_verts {
        vert.pos += *offset;
    }

    render_pass.set_vertex_buffer(0, vertex_buffer(&point_verts.to_vec(), device).slice(..));
    render_pass.set_index_buffer(
        index_buffer(vec![0, 1, 2, 1, 2, 3], &device).slice(..),
        wgpu::IndexFormat::Uint32,
    );
    render_pass.draw_indexed(0..6, 0, 0..1);

    point_verts
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
    wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("index Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        },
    )
}

fn raw_to_world_vert(
    mut vert: Vertex,
    bone: Option<&Bone>,
    camera: &Vec2,
    zoom: f32,
    tex_size: Vec2,
    aspect_ratio: f32,
    hard_scale: f32,
) -> Vertex {
    vert.pos *= hard_scale;

    if let Some(bone) = bone {
        let pivot_offset = tex_size * bone.pivot * hard_scale;
        vert.pos.x -= pivot_offset.x;
        vert.pos.y += pivot_offset.y;

        vert.pos *= bone.scale;

        // rotate verts
        vert.pos = utils::rotate(&vert.pos, bone.rot);

        // move verts with bone
        vert.pos += bone.pos;
    }

    // offset bone with camera
    vert.pos -= *camera;

    // adjust for zoom level
    vert.pos /= zoom;

    // adjust verts for aspect ratio
    vert.pos.x /= aspect_ratio;

    vert
}

fn world_to_raw_vert(
    mut vert: Vertex,
    bone: Option<&Bone>,
    camera: &Vec2,
    zoom: f32,
    tex_size: Vec2,
    aspect_ratio: f32,
    hard_scale: f32,
) -> Vertex {
    vert.pos.x *= aspect_ratio;

    vert.pos *= zoom;

    vert.pos += *camera;

    if let Some(bone) = bone {
        vert.pos -= bone.pos;

        vert.pos = utils::rotate(&vert.pos, -bone.rot);

        vert.pos /= bone.scale;

        let pivot_offset = tex_size * bone.pivot * hard_scale;
        vert.pos.x += pivot_offset.x;
        vert.pos.y -= pivot_offset.y;
    }

    vert.pos /= hard_scale;

    vert
}

fn draw_gridline(render_pass: &mut RenderPass, device: &Device, shared: &Shared) {
    render_pass.set_index_buffer(
        index_buffer([0, 1, 2].to_vec(), &device).slice(..),
        wgpu::IndexFormat::Uint32,
    );

    render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);

    let col = VertexColor::new(
        shared.config.colors.gridline.r as f32 / 255.,
        shared.config.colors.gridline.g as f32 / 255.,
        shared.config.colors.gridline.b as f32 / 255.,
        1.,
    );

    let width = 0.005 * shared.camera.zoom;
    let regular_color = VertexColor::new(col.r, col.g, col.b, 0.15);
    let highlight_color = VertexColor::new(col.r, col.g, col.b, 1.);

    // draw vertical lines
    let aspect_ratio = shared.window.y / shared.window.x;
    let mut x = (shared.camera.pos.x - shared.camera.zoom / aspect_ratio).round();
    let right_side = shared.camera.pos.x + shared.camera.zoom / aspect_ratio;
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
        draw_vertical_line(x, width, render_pass, device, shared, color);
        x += 1.;
    }

    // draw horizontal lines
    let mut y = (shared.camera.pos.y - shared.camera.zoom).round();
    let top_side = shared.camera.pos.y + shared.camera.zoom;
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
        draw_horizontal_line(y, width, render_pass, device, shared, color);
        y += 1.;
    }
}

pub fn draw_horizontal_line(
    y: f32,
    width: f32,
    render_pass: &mut RenderPass,
    device: &Device,
    shared: &Shared,
    color: VertexColor,
) {
    let edge = shared.camera.zoom * 5.;
    let camera_pos = shared.camera.pos;
    let camera_zoom = shared.camera.zoom;
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: (Vec2::new(camera_pos.x - edge, y) - camera_pos) / camera_zoom,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(camera_pos.x, width + y) - camera_pos) / camera_zoom,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(camera_pos.x + edge, y) - camera_pos) / camera_zoom,
            color,
            ..Default::default()
        },
    ];
    render_pass.set_vertex_buffer(0, vertex_buffer(&vertices, device).slice(..));
    render_pass.draw_indexed(0..3, 0, 0..1);
}

pub fn draw_vertical_line(
    x: f32,
    width: f32,
    render_pass: &mut RenderPass,
    device: &Device,
    shared: &Shared,
    color: VertexColor,
) {
    let aspect_ratio = shared.window.y / shared.window.x;
    let edge = shared.camera.zoom * 5.;
    let camera_pos = shared.camera.pos;
    let camera_zoom = shared.camera.zoom;
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: (Vec2::new(x, camera_pos.y - edge) - camera_pos) / camera_zoom * aspect_ratio,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(width + x, camera_pos.y) - camera_pos) / camera_zoom * aspect_ratio,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(x, camera_pos.y + edge) - camera_pos) / camera_zoom * aspect_ratio,
            color,
            ..Default::default()
        },
    ];
    render_pass.set_vertex_buffer(0, vertex_buffer(&vertices, device).slice(..));
    render_pass.draw_indexed(0..3, 0, 0..1);
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
