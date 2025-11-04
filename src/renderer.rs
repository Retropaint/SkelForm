//! Core rendering logic, abstracted from the rest of WGPU.

use crate::*;
use armature_window::find_bone;
use image::{DynamicImage, GenericImageView};
use spade::Triangulation;
use wgpu::{BindGroup, BindGroupLayout, Device, IndexFormat, Queue, RenderPass};
use winit::keyboard::KeyCode;

// todo:
// improve vert space conversions. This macro is starting to become the bane of my existence
#[rustfmt::skip]
macro_rules! con_vert {
    ($vert:expr, $bone:expr, $tex_size:expr, $cam_pos:expr, $cam_zoom:expr) => {
        raw_to_world_vert($vert, Some(&$bone), &$cam_pos, $cam_zoom, $tex_size, 1., Vec2::default())
    };
}

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    if shared.window == Vec2::ZERO {
        return;
    }

    shared.ui.set_state(UiState::Scaling, false);
    shared.ui.set_state(UiState::Rotating, false);

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

    for b in 0..shared.armature.bones.len() {
        macro_rules! bone {
            () => {
                shared.armature.bones[b]
            };
        }

        let tex = shared.armature.get_current_tex(bone!().id);

        if tex == None || bone!().vertices.len() != 0 {
            continue;
        }

        (bone!().vertices, bone!().indices) = create_tex_rect(&tex.unwrap().size);
    }

    // runtime:
    // armature bones should normally be mutable to animation for blending,
    // but that's not ideal when editing
    let mut animated_bones = shared.armature.bones.clone();

    let is_any_anim_playing = shared
        .armature
        .animations
        .iter()
        .find(|anim| anim.elapsed != None)
        != None;

    if is_any_anim_playing {
        // runtime: playing animations (single & simultaneous)
        for a in 0..shared.armature.animations.len() {
            let anim = &mut shared.armature.animations[a];
            if anim.elapsed == None {
                continue;
            }
            let frame = anim.set_frame();
            animated_bones = shared.armature.animate(a, frame, Some(&animated_bones));
        }
    } else if shared.ui.anim.open
        && shared.ui.anim.selected != usize::MAX
        && shared.ui.anim.selected_frame != -1
    {
        // display the selected animation's frame
        animated_bones =
            shared
                .armature
                .animate(shared.ui.anim.selected, shared.ui.anim.selected_frame, None);
    }

    // runtime: armature bones should be immutable to rendering
    let mut temp_bones: Vec<Bone> = animated_bones.clone();

    construction(&mut temp_bones, &animated_bones);

    // sort bones by highest zindex first, so that hover logic will pick the top-most one
    temp_bones.sort_by(|a, b| b.zindex.cmp(&a.zindex));

    let mut hover_bone_id = -1;

    // many fight for spot of newest vertex; only one will emerge victorious.
    let mut new_vert: Option<Vertex> = None;
    let mut removed_vert = false;

    // pre-draw bone setup
    for b in 0..temp_bones.len() {
        let tex = shared.armature.get_current_tex(temp_bones[b].id);
        if tex == None || shared.armature.is_bone_hidden(temp_bones[b].id) {
            continue;
        }

        let tex_size = tex.unwrap().size;
        let cam = &shared.camera;
        for v in 0..temp_bones[b].vertices.len() {
            let tb = &mut temp_bones[b];
            let mut vert = con_vert!(tb.vertices[v], tb, tex_size, cam.pos, cam.zoom);
            vert.pos.x *= shared.aspect_ratio();
            tb.world_verts.push(vert);
        }

        // check if cursor is on an opaque pixel of this bone's texture
        let tb = &temp_bones[b];
        let selected_mesh = !shared.ui.showing_mesh
            || shared.ui.showing_mesh && shared.selected_bone().unwrap().id == tb.id;
        if hover_bone_id == -1 && !shared.input.left_down && !shared.input.on_ui && selected_mesh {
            let wv = &temp_bones[b].world_verts;
            for (i, chunk) in temp_bones[b].indices.chunks_exact(3).enumerate() {
                let c0 = chunk[0] as usize;
                let c1 = chunk[1] as usize;
                let c2 = chunk[2] as usize;

                let bary = tri_point(&mouse_world_vert.pos, &wv[c0].pos, &wv[c1].pos, &wv[c2].pos);
                if bary.0 == -1. {
                    continue;
                }

                let uv = wv[c0].uv * bary.3 + wv[c1].uv * bary.1 + wv[c2].uv * bary.2;

                let bones = &shared.armature.bones;
                let v = &bones.iter().find(|bone| bone.id == tb.id).unwrap().vertices;
                let pos = v[c0].pos * bary.3 + v[c1].pos * bary.1 + v[c2].pos * bary.2;

                if shared.input.left_clicked && shared.ui.showing_mesh && new_vert == None {
                    new_vert = Some(Vertex {
                        pos,
                        uv,
                        ..Default::default()
                    });
                }

                if shared.ui.showing_mesh && shared.input.right_clicked && !removed_vert {
                    let bone = &mut shared.selected_bone_mut().unwrap();
                    let mut ids = vec![];
                    for i in &bone.indices {
                        ids.push(bone.vertices[*i as usize].id);
                    }
                    bone.indices.remove(i * 3);
                    bone.indices.remove(i * 3);
                    bone.indices.remove(i * 3);
                    removed_vert = true;
                    break;
                }

                let tex = shared.armature.get_current_tex(temp_bones[b].id);

                let img = &tex.unwrap().image;
                let pos = Vec2::new(
                    (uv.x * img.width() as f32).min(img.width() as f32 - 1.),
                    (uv.y * img.height() as f32).min(img.height() as f32 - 1.),
                );
                let pixel_alpha = tex.unwrap().image.get_pixel(pos.x as u32, pos.y as u32).0[3];
                if pixel_alpha == 255 && !shared.ui.showing_mesh {
                    hover_bone_id = temp_bones[b].id;
                    break;
                }
            }
        }

        let mut click_on_hover_id = temp_bones[b].id;
        if !shared.config.exact_bone_select {
            // QoL: select parent of textured bone if it's called 'Texture'
            // this is because most textured bones are meant to represent their parents
            let parents = shared.armature.get_all_parents(temp_bones[b].id);
            if parents.len() != 0 && temp_bones[b].name.to_lowercase() == "texture" {
                click_on_hover_id = parents[0].id;
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
                shared.ui.selected_bone_ids = vec![];

                // unfold all parents that lead to this bone, so it's visible in the hierarchy
                let parents = shared.armature.get_all_parents(click_on_hover_id);
                for parent in &parents {
                    shared.armature.find_bone_mut(parent.id).unwrap().folded = false;
                }
            }
        }
    }

    // runtime: sort bones by z-index for drawing
    temp_bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    for bone in &mut temp_bones {
        let tex = shared.armature.get_current_tex(bone.id);
        if tex == None || shared.armature.is_bone_hidden(bone.id) {
            continue;
        }

        if shared.ui.showing_mesh && shared.selected_bone().unwrap().id == bone.id {
            continue;
        }

        let bind_group = &tex.unwrap().bind_group;
        draw(
            &bind_group,
            &bone.world_verts,
            &bone.indices,
            render_pass,
            device,
        );
    }

    // draw inverse kinematics arrows

    // todo:
    // only draw arrows for the selected set of bones.
    // currently it shows all when any are selected.
    if shared.selected_bone() != None
        && shared.armature.bone_eff(shared.selected_bone().unwrap().id) != JointEffector::None
    {
        for bone in &temp_bones {
            if shared.armature.bone_eff(bone.id) == JointEffector::None
                || shared.armature.bone_eff(bone.id) == JointEffector::End
            {
                continue;
            }
            let mut arrow = Bone {
                pos: bone.pos,
                rot: bone.rot,
                scale: Vec2::new(2., 2.),
                ..Default::default()
            };
            let tex_size = Vec2::new(61., 48.);
            (arrow.vertices, arrow.indices) = create_tex_rect(&tex_size);
            for v in 0..4 {
                let mut new_vert = raw_to_world_vert(
                    arrow.vertices[v],
                    Some(&arrow),
                    &shared.camera.pos,
                    shared.camera.zoom,
                    tex_size,
                    shared.aspect_ratio(),
                    Vec2::new(0., 0.5),
                );
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

    if shared.ui.showing_mesh {
        let id = shared.selected_bone().unwrap().id;
        let bone = temp_bones.iter_mut().find(|bone| bone.id == id).unwrap();
        let bind_group = &shared.armature.get_current_tex(bone.id).unwrap().bind_group;

        let vert = &bone.world_verts;
        draw(&bind_group, &vert, &bone.indices, render_pass, device);

        render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);

        let vert = mouse_world_vert;
        vert_lines(bone, shared, &vert, render_pass, device, &mut new_vert);
        bone_vertices(&bone, shared, render_pass, device, &bone.world_verts);
    }

    if !shared.ui.setting_weight_verts {
        if let Some(mut vert) = new_vert {
            shared.undo_actions.push(Action {
                action: ActionType::Bone,
                id: shared.selected_bone().unwrap().id,
                bones: vec![shared.selected_bone().unwrap().clone()],
                ..Default::default()
            });
            let bone_mut = shared.selected_bone_mut().unwrap();
            let ids = bone_mut.vertices.iter().map(|v| v.id as i32).collect();
            vert.id = generate_id(ids) as u32;
            bone_mut.vertices.push(vert);
            bone_mut.vertices = sort_vertices(bone_mut.vertices.clone());
            bone_mut.indices = triangulate(&bone_mut.vertices);
        }
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
        let bone = find_bone(&temp_bones, shared.selected_bone().unwrap().id).unwrap();
        let cam = shared.camera.pos;
        draw_point(
            &Vec2::ZERO,
            &shared,
            render_pass,
            device,
            bone,
            color,
            cam,
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
            let bone = &temp_bones.iter().find(|bone| bone.id == bone_id).unwrap();
            drag_vertex(shared, bone, &temp_bones, vert);
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
        let is_end = shared.edit_mode == EditMode::Rotate
            && shared.armature.bone_eff(bone.id) == JointEffector::End;
        ik_disabled = is_end
            || (bone.ik_disabled || shared.armature.bone_eff(bone.id) == JointEffector::None);
    }

    if shared.ui.showing_mesh || !ik_disabled {
        return;
    }

    // editing bone
    if shared.input.on_ui || shared.ui.has_state(UiState::PolarModal) {
        shared.editing_bone = false;
    } else if shared.ui.selected_bone_idx != usize::MAX
        && shared.input.left_down
        && hover_bone_id == -1
        && shared.input.down_dur > 5
    {
        if shared.edit_mode == EditMode::Rotate {
            let mut mouse = utils::screen_to_world_space(shared.input.mouse, shared.window);
            mouse.x *= shared.aspect_ratio();
            let bone = find_bone(&temp_bones, shared.selected_bone().unwrap().id).unwrap();
            let center = Vertex {
                pos: bone.pos,
                ..Default::default()
            };
            let center_world = raw_to_world_vert(
                center,
                None,
                &shared.camera.pos,
                shared.camera.zoom,
                Vec2::ZERO,
                shared.aspect_ratio(),
                Vec2::new(0.5, 0.5),
            );
            draw_line(center_world.pos, mouse, shared, render_pass, &device);
        }

        // save bone/animation for undo
        if !shared.editing_bone {
            shared.save_edited_bone();
            //shared.armature.autosave();
            *shared.saving.lock().unwrap() = Saving::Autosaving;
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
    construction(&mut temp_bones, &shared.armature.bones);
    temp_bones.sort_by(|a, b| a.zindex.cmp(&b.zindex));

    let zoom = 1000.;

    for b in 0..temp_bones.len() {
        if shared.armature.get_current_tex(temp_bones[b].id) == None {
            continue;
        }
        let set = shared.armature.get_current_set(temp_bones[b].id);
        if set == None
            || temp_bones[b].tex_idx > set.unwrap().textures.len() as i32 - 1
            || shared.armature.is_bone_hidden(temp_bones[b].id)
        {
            continue;
        }

        let tex_size = set.unwrap().textures[temp_bones[b].tex_idx as usize].size;
        for v in 0..temp_bones[b].vertices.len() {
            let tb = &temp_bones[b];
            let mut new_vert = con_vert!(tb.vertices[v], tb, tex_size, Vec2::default(), zoom);
            new_vert.pos.x /= shared.window.x / shared.window.y;
            new_vert.add_color = VertexColor::new(0., 0., 0., 0.);
            temp_bones[b].world_verts.push(new_vert);
        }

        let bind_group = &shared
            .armature
            .get_current_tex(temp_bones[b].id)
            .unwrap()
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

        // IK is iterated multiple times over for accuracy
        // runtimes could adjust this, or make it customizable
        for _ in 0..10 {
            inverse_kinematics(&mut joints, target.unwrap().pos);
        }

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
        for v in 0..bones[b].vertices.len() {
            #[rustfmt::skip]
            macro_rules! vert {() =>{ bones[b].vertices[v] }}

            let init_pos = vert!().pos;

            inherit_vert(&mut vert!(), &bone);

            for weight in bones[b].weights.clone() {
                if !weight.vert_ids.contains(&(vert!().id as i32)) {
                    continue;
                }

                let bone_id = weight.bone_id;
                let weight_bone = bones.iter().find(|b| b.id == bone_id).unwrap().clone();

                vert!().pos = init_pos;
                inherit_vert(&mut vert!(), &weight_bone);
            }
        }
    }
}

pub fn inherit_vert(vert: &mut Vertex, bone: &Bone) {
    vert.pos *= bone.scale;
    vert.pos = utils::rotate(&vert.pos, bone.rot);
    vert.pos += bone.pos;
}

// https://www.youtube.com/watch?v=NfuO66wsuRg
pub fn inverse_kinematics(bones: &mut Vec<Bone>, target: Vec2) {
    let root = bones[0].pos;

    let base_dir = (target - root).normalize();
    let base_angle = base_dir.y.atan2(base_dir.x);

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

        if b != 0 && b != bones.len() - 1 && bones[0].constraint != JointConstraint::None {
            // get local angle of joint
            let joint_dir = (prev_pos - bones[b].pos).normalize();
            let joint_angle = joint_dir.y.atan2(joint_dir.x) - base_angle;

            let const_min;
            let const_max;
            if bones[0].constraint == JointConstraint::Clockwise {
                const_min = -3.14;
                const_max = 0.;
            } else {
                const_min = 0.;
                const_max = 3.14;
            }

            // if joint angle is beyond constraint, rotate the hinge so it's on the opposite side
            if joint_angle > const_max || joint_angle < const_min {
                let rot_offset = -joint_angle * 2.;
                let rotated = utils::rotate(&(bones[b].pos - prev_pos), rot_offset);
                bones[b].pos = rotated + prev_pos;
            }
        }

        prev_pos = bones[b].pos;
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
            shared
                .armature
                .edit_bone($bone.id, &$element, $value, anim_id, anim_frame);
        };
    }

    let bone_center = raw_to_world_vert(
        Vertex {
            pos: bone.pos,
            ..Default::default()
        },
        None,
        &shared.camera.pos,
        shared.camera.zoom,
        Vec2::ZERO,
        shared.aspect_ratio(),
        Vec2::new(0.5, 0.5),
    );

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
            shared.ui.set_state(UiState::Rotating, true);

            let mut mouse = utils::screen_to_world_space(shared.input.mouse, shared.window);
            mouse.x *= shared.aspect_ratio();

            let dir = mouse - bone_center.pos;
            let rot = dir.y.atan2(dir.x);

            edit!(bone, AnimElement::Rotation, rot);
        }
        shared::EditMode::Scale => {
            shared.ui.set_state(UiState::Scaling, true);

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
        let idx = shared.ui.selected_weights;
        let mut col = if idx != -1
            && shared.selected_bone().unwrap().weights[idx as usize]
                .vert_ids
                .contains(&(world_verts[wv].id as i32))
        {
            VertexColor::YELLOW
        } else {
            VertexColor::GREEN
        };
        col.a = 0.5;
        let point = point!(wv, col);
        let mouse_on_it = utils::in_bounding_box(&shared.input.mouse, &point, &shared.window).1;

        if shared.input.on_ui || !mouse_on_it {
            continue;
        }

        point!(wv, VertexColor::WHITE);
        if shared.input.right_clicked {
            if world_verts.len() <= 4 {
                let str_vert_limit = &shared.loc("vert_limit");
                shared.ui.open_modal(str_vert_limit.to_string(), false);
            } else {
                let verts = &mut shared.selected_bone_mut().unwrap().vertices;
                verts.remove(wv);
                *verts = sort_vertices(verts.clone());
                shared.selected_bone_mut().unwrap().indices = triangulate(&verts);
                break;
            }
        }
        if !shared.ui.setting_weight_verts {
            if shared.input.left_pressed {
                shared.undo_actions.push(Action {
                    action: ActionType::Bone,
                    bones: vec![shared.selected_bone().unwrap().clone()],
                    id: shared.selected_bone().unwrap().id,
                    ..Default::default()
                });
                shared.dragging_verts = vec![wv];
                break;
            }
        } else if shared.input.left_clicked {
            let idx = shared.ui.selected_weights as usize;
            let vert_id = world_verts[wv].id;
            let weight = &mut shared.selected_bone_mut().unwrap().weights[idx];
            if let Some(idx) = weight.vert_ids.iter().position(|v| *v == vert_id as i32) {
                weight.vert_ids.remove(idx);
            } else {
                weight.vert_ids.push(vert_id as i32);
            }
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
    let mut added_vert = false;

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
        col.a = 0.3;

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

        let mut verts = vec![v0_top, v0_bot, v1_top, v1_bot];
        let indices = vec![0, 1, 2, 1, 2, 3];
        let add_color = VertexColor::new(0.2, 0.2, 0.2, 1.);

        let mut is_hovering = false;

        for (_, chunk) in indices.chunks_exact(3).enumerate() {
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
                let bones = &shared.armature.bones;
                let v = &bones.iter().find(|b| b.id == bone.id).unwrap().vertices;
                let wv0 = v[i0 as usize].pos;
                let wv1 = v[i1 as usize].pos;
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

        verts = vec![v0_top, v0_bot, v1_top, v1_bot];

        draw(&None, &verts, &indices, render_pass, device);
    }
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

pub fn drag_vertex(shared: &mut Shared, bone: &Bone, bones: &Vec<Bone>, vert_idx: usize) {
    let mouse_vel = shared.mouse_vel();
    let zoom = shared.camera.zoom;
    let vert_id = bone.vertices[vert_idx].id;
    let mut total_rot = bone.rot;
    for weight in &bone.weights {
        if !weight.vert_ids.contains(&(vert_id as i32)) {
            continue;
        }
        let weight_bone = bones.iter().find(|b| b.id == weight.bone_id).unwrap();
        total_rot += weight_bone.rot;
    }
    // offset weight rotations
    let vert_mut = &mut shared.selected_bone_mut().unwrap().vertices[vert_idx];
    vert_mut.pos -= utils::rotate(&(mouse_vel * zoom), -total_rot);
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
    render_pass: &mut RenderPass,
    device: &Device,
    bone: &Bone,
    color: VertexColor,
    camera: Vec2,
    rotation: f32,
) -> Vec<Vertex> {
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
        v.pos += bone.pos;
        v.pos = utils::rotate(&v.pos, rotation);
    }

    let mut point_verts = vec![];
    let ar = shared.aspect_ratio();
    let cam = &shared.camera;
    let pivot = Vec2::new(0.5, 0.5);
    for vert in temp_point_verts {
        let vert = raw_to_world_vert(vert, None, &camera, cam.zoom, Vec2::ZERO, ar, pivot);
        point_verts.push(vert);
    }

    for vert in &mut point_verts {
        vert.pos += *offset;
    }

    let range = vec![0, 1, 2, 1, 2, 3];
    render_pass.set_vertex_buffer(0, vertex_buffer(&point_verts.to_vec(), device).slice(..));
    render_pass.set_index_buffer(index_buffer(range, &device).slice(..), IndexFormat::Uint32);
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
    pivot: Vec2,
) -> Vertex {
    if bone != None {
        let pivot_offset = tex_size * pivot;
        vert.pos.x -= pivot_offset.x;
        vert.pos.y += pivot_offset.y;
    }

    // offset bone with camera
    vert.pos -= *camera;

    // adjust for zoom level
    vert.pos /= zoom;

    // adjust verts for aspect ratio
    vert.pos.x *= aspect_ratio;

    vert
}

fn draw_gridline(render_pass: &mut RenderPass, device: &Device, shared: &Shared) {
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

    let mut verts = vec![];
    let mut indices: Vec<u32> = vec![];
    let mut i: u32 = 0;

    // draw vertical lines
    let mut x = (shared.camera.pos.x - shared.camera.zoom / shared.aspect_ratio()).round();
    let right_side = shared.camera.pos.x + shared.camera.zoom / shared.aspect_ratio();
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

pub fn draw_horizontal_line(
    y: f32,
    width: f32,
    shared: &Shared,
    color: VertexColor,
) -> Vec<Vertex> {
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
    vertices
}

pub fn draw_vertical_line(x: f32, width: f32, shared: &Shared, color: VertexColor) -> Vec<Vertex> {
    let edge = shared.camera.zoom * 5.;
    let camera_pos = shared.camera.pos;
    let camera_zoom = shared.camera.zoom;
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: (Vec2::new(x, camera_pos.y - edge) - camera_pos) / camera_zoom
                * shared.aspect_ratio(),
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(width + x, camera_pos.y) - camera_pos) / camera_zoom
                * shared.aspect_ratio(),
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(x, camera_pos.y + edge) - camera_pos) / camera_zoom
                * shared.aspect_ratio(),
            color,
            ..Default::default()
        },
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
