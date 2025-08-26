//! Core rendering logic, abstracted from the rest of WGPU.

use crate::*;
use armature_window::find_bone;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};
use winit::keyboard::KeyCode;

macro_rules! con_vert {
    ($func:expr, $vert:expr, $bone:expr, $tex:expr, $cam_pos:expr, $cam_zoom:expr) => {
        $func(
            $vert,
            Some(&$bone),
            &$cam_pos,
            $cam_zoom,
            Some(&$tex),
            1.,
            1.,
        )
    };
}

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    if shared.generic_bindgroup != None {
        draw_gridline(render_pass, device, shared);
    }

    #[cfg(target_arch = "wasm32")]
    loaded();

    for bone in &mut shared.armature.bones {
        if bone.tex_set_idx != -1 && bone.vertices.len() == 0 {
            let tex_size = shared.armature.texture_sets[bone.tex_set_idx as usize].textures
                [bone.tex_idx as usize]
                .size;
            (bone.vertices, bone.indices) = create_tex_rect(&tex_size);
        }
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

    // For rendering purposes, bones need to have many of their attributes manipulated.
    // This is easier to do with a separate copy of them.
    let mut temp_bones: Vec<Bone> = bones.clone();

    let mut init_rot: std::collections::HashMap<i32, f32> = std::collections::HashMap::new();

    // first FK to construct the bones
    forward_kinematics(&mut temp_bones, std::collections::HashMap::new());

    // inverse kinematics
    for b in 0..temp_bones.len() {
        if !temp_bones[b].aiming {
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

        // apply IK on the joint copy, then apply it to the actual bones
        let target = (mouse_world * shared.camera.zoom) + shared.camera.pos;
        for _ in 0..10 {
            inverse_kinematics(&mut joints, target);
        }
        for joint in joints {
            init_rot.insert(joint.id, joint.rot);
        }
    }

    // re-construct bones, accounting for IK
    temp_bones = bones.clone();
    forward_kinematics(&mut temp_bones, init_rot);

    // sort bones by z-index for drawing
    temp_bones.sort_by(|a, b| a.zindex.total_cmp(&b.zindex));

    let mut hovering_vert = usize::MAX;

    let mut selected_bone_world_verts: Vec<Vertex> = vec![];

    // draw bone
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

        let mut world_verts: Vec<Vertex> = vec![];
        for vert in &temp_bones[b].vertices {
            let mut new_vert = con_vert!(
                raw_to_world_vert,
                *vert,
                temp_bones[b],
                set.textures[temp_bones[b].tex_idx as usize],
                shared.camera.pos,
                shared.camera.zoom
            );
            new_vert.pos.x /= shared.window.x / shared.window.y;
            world_verts.push(new_vert);
        }

        let selected = shared.selected_bone() != None
            && temp_bones[b].id == shared.selected_bone().unwrap().id;

        if selected {
            selected_bone_world_verts = world_verts.clone();
        }

        draw_bone(&temp_bones[b], render_pass, device, &world_verts, shared);

        render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);
        if shared.ui.editing_mesh && b == shared.ui.selected_bone_idx {
            hovering_vert =
                bone_vertices(&temp_bones[b], shared, render_pass, device, &world_verts);
        }
    }

    if shared.selected_bone() != None {
        draw_point(
            &Vec2::ZERO,
            &shared,
            render_pass,
            device,
            &find_bone(&temp_bones, shared.selected_bone().unwrap().id).unwrap(),
            VertexColor::new(0., 255., 0., 0.5),
            shared.camera.pos,
            0.,
        );
    }

    if shared.input.mouse_left == -1 {
        shared.dragging_vert = usize::MAX;
    } else if shared.dragging_vert != usize::MAX {
        drag_vertex(
            shared,
            shared.dragging_vert,
            &temp_bones[shared.ui.selected_bone_idx].pos,
        );
        return;
    }

    if shared.selected_bone() != None
        && shared.ui.editing_mesh
        && shared.selected_bone().unwrap().vertices.len() > 0
        && hovering_vert == usize::MAX
        && !shared.input.on_ui
        && shared.dragging_vert == usize::MAX
    {
        draw_hover_triangle(shared, render_pass, device, &selected_bone_world_verts);
    }

    if shared.input.mouse_left == -1 && shared.input.mouse_right == -1 {
        shared.editing_bone = false;
        return;
    }

    if let Some(aimed_bone) = shared.armature.bones.iter_mut().find(|bone| bone.aiming) {
        aimed_bone.aiming = false;
    }

    // mouse related stuff

    // move camera
    if (shared.input.is_pressing(KeyCode::SuperLeft)
        || shared.input.mouse_right > 0
        || shared.ui.selected_bone_idx == usize::MAX)
        && !shared.input.on_ui
    {
        shared.camera.pos += shared.mouse_vel() * shared.camera.zoom;
        return;
    }

    // editing bone
    if shared.input.on_ui || shared.ui.has_state(UiState::PolarModal) {
        shared.editing_bone = false;
    } else if shared.ui.selected_bone_idx != usize::MAX && shared.input.is_holding_click() {
        // save bone/animation for undo
        if !shared.editing_bone {
            shared.save_edited_bone();
            shared.armature.autosave();
            shared.editing_bone = true;
        }

        shared.cursor_icon = egui::CursorIcon::Crosshair;

        let bone = find_bone(&temp_bones, shared.selected_bone().unwrap().id).unwrap();
        edit_bone(shared, bone, &temp_bones);
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
    let mut tip_pos = bones
        .iter_mut()
        .find(|bone| bone.joint_effector == JointEffector::End)
        .unwrap()
        .pos;
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

fn get_distance(a: Vec2, b: Vec2) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

fn find_closest_vert(point_vert: Vertex, verts: &Vec<Vertex>, exception: usize) -> usize {
    let mut closest = 0;
    let mut distance = std::f32::INFINITY;
    for i in 0..verts.len() {
        let current_distance = get_distance(point_vert.pos, verts[i].pos);
        if current_distance < distance && i != exception {
            distance = current_distance;
            closest = i;
        }
    }
    closest
}

fn draw_hover_triangle(
    shared: &mut Shared,
    render_pass: &mut RenderPass,
    device: &Device,
    world_verts: &Vec<Vertex>,
) {
    macro_rules! bone {
        () => {
            shared.selected_bone().unwrap()
        };
    }
    let tex = &shared.armature.texture_sets[bone!().tex_set_idx as usize].textures
        [bone!().tex_idx as usize];

    // create vert on cursor
    let mut mouse_world_vert = Vertex {
        pos: utils::screen_to_world_space(shared.input.mouse, shared.window),
        ..Default::default()
    };
    let mut mouse_vert = con_vert!(
        world_to_raw_vert,
        mouse_world_vert,
        bone!(),
        tex,
        shared.camera.pos,
        shared.camera.zoom
    );
    mouse_world_vert.pos.x *= shared.window.y / shared.window.x;

    // get the 2 closest verts to the mouse
    let closest_vert1 = find_closest_vert(mouse_world_vert, world_verts, usize::MAX);
    let closest_vert2 = find_closest_vert(mouse_world_vert, world_verts, closest_vert1);

    // draw hover triangle
    render_pass.set_vertex_buffer(
        0,
        vertex_buffer(
            &vec![
                world_verts[closest_vert1],
                world_verts[closest_vert2],
                mouse_world_vert,
            ],
            device,
        )
        .slice(..),
    );
    render_pass.set_index_buffer(
        index_buffer(vec![0, 1, 2], &device).slice(..),
        wgpu::IndexFormat::Uint32,
    );
    render_pass.draw_indexed(0..3, 0, 0..1);

    if shared.selected_bone() == None
        || !shared.input.clicked()
        || shared.input.on_ui
        || !shared.ui.editing_mesh
    {
        return;
    }

    shared.undo_actions.push(Action {
        action: ActionEnum::Bone,
        bones: vec![bone!().clone()],
        id: bone!().id,
        ..Default::default()
    });

    // add new vertex
    mouse_vert.uv = (world_verts[closest_vert1].uv + world_verts[closest_vert2].uv) / 2.;
    shared
        .selected_bone_mut()
        .unwrap()
        .vertices
        .push(mouse_vert);

    shared.selected_bone_mut().unwrap().vertices = sort_vertices(bone!().vertices.clone());

    // get the new vert's index, then use it as the base
    let mut vert_idx = 0;
    for (i, v) in bone!().vertices.iter().enumerate() {
        if v.pos == mouse_vert.pos {
            vert_idx = i;
            break;
        }
    }

    shared.selected_bone_mut().unwrap().indices =
        setup_indices(&shared.selected_bone().unwrap().vertices, vert_idx as i32);
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

/// generate a usable index array for the supplied vertices
///
/// don't forget to use sort_vertices() first!
pub fn setup_indices(verts: &Vec<Vertex>, base: i32) -> Vec<u32> {
    //return lyon_poly(verts);
    //return triangulate(verts);

    let len = verts.len();
    let mut indices: Vec<u32> = vec![];
    for v in 0..verts.len() {
        if v > verts.len() - 1 {
            break;
        }
        let mut v1 = v + base as usize;
        if v1 > len - 1 {
            v1 -= len;
        }

        let mut v2 = v + base as usize + 1;
        if v2 > len - 1 {
            v2 -= len;
        }

        // exclude redundant verts
        if base as usize != v1 && v1 != v2 && v2 != base as usize {
            indices.push(base as u32);
            indices.push(v1 as u32);
            indices.push(v2 as u32);
        }
    }

    indices
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

pub fn draw_bone(
    bone: &Bone,
    render_pass: &mut RenderPass,
    device: &Device,
    world_verts: &Vec<Vertex>,
    shared: &Shared,
) {
    //render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);
    render_pass.set_bind_group(
        0,
        &shared.armature.texture_sets[bone.tex_set_idx as usize].textures[bone.tex_idx as usize]
            .bind_group,
        &[],
    );
    render_pass.set_vertex_buffer(0, vertex_buffer(&world_verts, device).slice(..));
    render_pass.set_index_buffer(
        index_buffer(bone.indices.to_vec(), &device).slice(..),
        wgpu::IndexFormat::Uint32,
    );
    render_pass.draw_indexed(0..bone.indices.len() as u32, 0, 0..1);
}

pub fn bone_vertices(
    bone: &Bone,
    shared: &mut Shared,
    render_pass: &mut RenderPass,
    device: &Device,
    world_verts: &Vec<Vertex>,
) -> usize {
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

    let mut hovering_vert = usize::MAX;

    for wv in 0..world_verts.len() {
        let point = point!(wv, VertexColor::GREEN);
        let mouse_on_it = utils::in_bounding_box(&shared.input.mouse, &point, &shared.window).1;
        if shared.input.on_ui || !mouse_on_it || shared.dragging_vert != usize::MAX {
            continue;
        }

        hovering_vert = wv;
        point!(wv, VertexColor::WHITE);
        if shared.input.right_clicked() && world_verts.len() > 4 {
            let verts = &mut shared.selected_bone_mut().unwrap().vertices;
            verts.remove(wv);
            *verts = sort_vertices(verts.clone());
            shared.selected_bone_mut().unwrap().indices =
                setup_indices(&verts, verts.len() as i32 - 1);
            break;
        }
        if shared.input.is_clicking() {
            shared.undo_actions.push(Action {
                action: ActionEnum::Bone,
                bones: vec![shared.selected_bone().unwrap().clone()],
                id: shared.selected_bone().unwrap().id,
                ..Default::default()
            });
            shared.dragging_vert = wv;
            break;
        }
    }

    hovering_vert
}

pub fn drag_vertex(shared: &mut Shared, vert_idx: usize, bone_pos: &Vec2) {
    let mut bone = shared.selected_bone().unwrap().clone();

    // vertex conversion should consider the animated bone position
    if shared.ui.is_animating() {
        bone.pos = *bone_pos;
    }

    // when moving a vertex, it must be interpreted in world coords first to align the with the mouse
    let mut world_vert = con_vert!(
        raw_to_world_vert,
        bone.vertices[vert_idx],
        bone,
        shared.armature.texture_sets[bone.tex_set_idx as usize].textures[bone.tex_idx as usize],
        shared.camera.pos,
        shared.camera.zoom
    );

    // now that it's in world coords, it can follow the mouse
    world_vert.pos = utils::screen_to_world_space(shared.input.mouse, shared.window);

    // convert back to normal coords
    let vert_pos = con_vert!(
        world_to_raw_vert,
        world_vert,
        bone,
        shared.armature.texture_sets[bone.tex_set_idx as usize].textures[bone.tex_idx as usize],
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
        shared.dragging_vert as i32,
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
            color: VertexColor::default(),
        },
        Vertex {
            pos: Vec2::new(tex_size.x, -tex_size.y),
            uv: Vec2::new(1., 1.),
            color: VertexColor::default(),
        },
        Vertex {
            pos: Vec2::new(0., -tex_size.y),
            uv: Vec2::new(0., 1.),
            color: VertexColor::default(),
        },
    ];
    verts = sort_vertices(verts.clone());
    let indices = setup_indices(&verts, verts.len() as i32 - 1);
    (verts, indices)
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
    if shared.generic_bindgroup == None {
        return vec![];
    }

    let point_size = 10.;
    let mut temp_point_verts: [Vertex; 4] = [
        Vertex {
            pos: Vec2::new(-point_size, point_size),
            uv: Vec2::new(1., 0.),
            color,
        },
        Vertex {
            pos: Vec2::new(point_size, point_size),
            uv: Vec2::new(0., 1.),
            color,
        },
        Vertex {
            pos: Vec2::new(-point_size, -point_size),
            uv: Vec2::new(0., 0.),
            color,
        },
        Vertex {
            pos: Vec2::new(point_size, -point_size),
            uv: Vec2::new(1., 1.),
            color,
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
            None,
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
    tex: Option<&Texture>,
    aspect_ratio: f32,
    hard_scale: f32,
) -> Vertex {
    vert.pos *= hard_scale;

    if let Some(bone) = bone {
        let pivot_offset = tex.unwrap().size * bone.pivot * hard_scale;
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
    tex: Option<&Texture>,
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

        let pivot_offset = tex.unwrap().size * bone.pivot * hard_scale;
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

    let width = 0.005 * shared.camera.zoom;
    let regular_color = VertexColor::new(0.5, 0.5, 0.5, 0.25);
    let highlight_color = VertexColor::new(0.7, 0.7, 0.7, 1.);

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

// pub fn triangulate(verts: &Vec<Vertex>) -> Vec<u32> {
//     let mut poly: Vec<geo::Coord> = vec![];
//     for vert in verts {
//         poly.push(geo::Coord {
//             x: vert.pos.x as f64,
//             y: vert.pos.y as f64,
//         });
//     }
//     let square_polygon = geo::Polygon::new(geo::LineString(poly), vec![]);
//     let indices = square_polygon.earcut_triangles_raw().triangle_indices;
//     let mut u32_indices: Vec<u32> = vec![];
//     for index in &indices {
//         u32_indices.push(*index as u32);
//     }
//     u32_indices
// }

// pub fn lyon_poly(verts: &Vec<Vertex>) -> Vec<u32> {
//     let mut raw_points: Vec<[f32; 2]> = vec![];
//     for vert in verts {
//         raw_points.push([vert.pos.x, vert.pos.y]);
//     }

//     //raw_points.push([0., 0.]);
//     //raw_points.push([1., 0.]);
//     //raw_points.push([1., 1.]);
//     //raw_points.push([0., 1.]);

//     println!("{:?}", raw_points);

//     // Convert to lyon Points
//     let points: Vec<lyon::math::Point> = raw_points
//         .iter()
//         .map(|&[x, y]| lyon::math::point(x, y))
//         .collect();

//     // Create path builder
//     let mut builder = lyon::path::Path::builder();
//     builder.begin(points[0]);
//     for p in &points[1..] {
//         builder.line_to(*p);
//     }
//     builder.end(true); // Close the polygon

//     let path = builder.build();

//     // Vertex + index buffers for tessellation result
//     let mut geometry: lyon::tessellation::VertexBuffers<lyon::math::Point, u16> =
//         lyon::tessellation::VertexBuffers::new();

//     let mut tessellator = lyon::tessellation::FillTessellator::new();
//     let result = tessellator.tessellate_path(
//         &path,
//         &lyon::tessellation::FillOptions::default(),
//         &mut lyon::tessellation::BuffersBuilder::new(
//             &mut geometry,
//             |vertex: lyon::tessellation::FillVertex| vertex.position(),
//         ),
//     );

//     match result {
//         Ok(_) => {
//             println!("Generated {} triangles", geometry.indices.len() / 3);
//             let mut indices = vec![];
//             for tri in geometry.indices.chunks(3) {
//                 println!(
//                     "Triangle: {:?} {:?} {:?}",
//                     geometry.vertices[tri[0] as usize],
//                     geometry.vertices[tri[1] as usize],
//                     geometry.vertices[tri[2] as usize],
//                 );
//                 for v in 0..verts.len() {
//                     if (geometry.vertices[tri[0] as usize].x == verts[v].pos.x
//                         && geometry.vertices[tri[0] as usize].y == verts[v].pos.y)
//                         || (geometry.vertices[tri[1] as usize].x == verts[v].pos.x
//                             && geometry.vertices[tri[1] as usize].y == verts[v].pos.y)
//                         || (geometry.vertices[tri[2] as usize].x == verts[v].pos.x
//                             && geometry.vertices[tri[2] as usize].y == verts[v].pos.y)
//                     {
//                         indices.push(v as u32);
//                     }
//                 }
//             }
//             return indices;
//         }
//         Err(e) => eprintln!("Tessellation error: {:?}", e),
//     }

//     vec![]
// }
