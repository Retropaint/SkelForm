//! Core rendering logic, abstracted from the rest of WGPU.

use std::f32::consts::PI;

use crate::{
    input,
    shared::{Bone, Shared, Texture, Vec2, Vertex},
    utils, AnimBone,
};
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};
use winit::keyboard::KeyCode;

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    let mut bones = shared.armature.bones.clone();
    if shared.animating && shared.ui.anim.selected != usize::MAX {
        bones = shared.animate(shared.ui.anim.selected, shared.ui.anim.selected_frame);
    }

    let mut verts = vec![];

    // For rendering purposes, bones need to have many of their attributes manipulated.
    // This is easier to do with a separate copy of them.
    let mut temp_bones: Vec<Bone> = vec![];
    for b in &mut bones {
        temp_bones.push(b.clone());
    }

    // using while loop to prevent borrow issues
    for i in 0..temp_bones.len() {
        if temp_bones[i].tex_idx == usize::MAX {
            continue;
        }

        // get parent bone
        let mut p = Bone::default();
        p.scale.x = 1.;
        p.scale.y = 1.;
        for b in &temp_bones {
            if b.id == temp_bones[i].parent_id {
                p = b.clone();
            }
        }

        temp_bones[i].rot += p.rot;
        temp_bones[i].scale *= p.scale;

        // adjust bone's position based on parent's scale
        temp_bones[i].pos *= p.scale;

        // rotate such that it will orbit the parent once it's position is inherited
        temp_bones[i].pos = utils::rotate(&temp_bones[i].pos, p.rot);

        // inherit position from parent
        temp_bones[i].pos += p.pos;

        // generate the vertices to be used later
        let this_verts = rect_verts(
            &temp_bones[i],
            &shared.camera.pos,
            shared.zoom,
            &shared.armature.textures[temp_bones[i].tex_idx],
            shared.window.x / shared.window.y,
        );
        verts.push(this_verts);
    }

    let mut hovered_bone = -1;
    let mut hovered_bone_verts: Vec<Vertex> = vec![];

    // Check for the bone being hovered on.
    // This has to be in reverse (for now) since bones are rendered in ascending order of the array,
    // so it visually makes sense to click the one that shows in front.
    for i in (0..temp_bones.len()).rev() {
        if shared.armature.bones[i].tex_idx == usize::MAX {
            continue;
        }
        let is_in_box: bool;
        (hovered_bone_verts, is_in_box) =
            utils::in_bounding_box(&shared.input.mouse, &verts[i], &shared.window);
        if is_in_box {
            // highlight bone for selection if not already selected
            if shared.selected_bone_idx != i {
                hovered_bone = i as i32;

                // select if left clicked
                if shared.input.mouse_left == 0 {
                    shared.selected_bone_idx = i;
                }
            }
            break;
        }
    }

    // finally, draw the bones
    for (i, b) in temp_bones.iter().enumerate() {
        if b.tex_idx == usize::MAX {
            continue;
        }

        // draw the hovering highlight section
        if hovered_bone as usize == i {
            render_pass.set_bind_group(0, &shared.highlight_bindgroup, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer(&hovered_bone_verts, device).slice(..));
            render_pass.set_index_buffer(
                index_buffer([0, 1, 2, 3, 0, 1].to_vec(), &device).slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        // draw bone
        render_pass.set_bind_group(0, &shared.bind_groups[b.tex_idx], &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer(&verts[i], device).slice(..));
        render_pass.set_index_buffer(
            index_buffer([0, 1, 2, 0, 1, 3].to_vec(), &device).slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
    }

    // mouse inputs
    if !shared.input.on_ui {
        if !input::is_pressing(KeyCode::SuperLeft, &shared) {
            edit_bone_with_mouse(shared, bones);
        } else if shared.input.mouse_left != -1 && shared.selected_bone_idx != usize::MAX {
            if shared.input.mouse_left == -1 {
                shared.input.initial_points = vec![];
            } else {
                // move camera if holding mod key
                if shared.input.initial_points.len() == 0 {
                    shared.camera.initial_pos = shared.camera.pos;
                    shared.input.initial_points.push(shared.input.mouse);
                }

                let mouse_world = utils::screen_to_world_space(shared.input.mouse, shared.window);
                let initial_world =
                    utils::screen_to_world_space(shared.input.initial_points[0], shared.window);
                shared.camera.pos =
                    shared.camera.initial_pos - (mouse_world - initial_world) * shared.zoom;
            }
        }
    }
}

pub fn edit_bone_with_mouse(shared: &mut Shared, bones: Vec<Bone>) {
    if shared.armature.bones[shared.selected_bone_idx].tex_idx == usize::MAX {
        return;
    }

    // translation
    if shared.edit_mode == 0 {
        shared.cursor_icon = egui::CursorIcon::Move;

        // modify either the armature's, or animation keyframe's bone
        if shared.animating && shared.ui.anim.selected != usize::MAX {
            check_if_in_keyframe(shared.selected_bone().id, shared);
            let pos = bones[shared.selected_bone_idx].pos;
            shared.selected_anim_bone_mut().unwrap().pos = shared.move_with_mouse(&pos, true);
        } else {
            let pos = shared.selected_bone().pos;
            shared.selected_bone_mut().pos = shared.move_with_mouse(&pos, true);
        }
    // rotation
    } else if shared.edit_mode == 1 {
        shared.selected_bone_mut().rot = (shared.input.mouse.x / shared.window.x) * PI * 2.;
        if shared.animating && shared.ui.anim.selected != usize::MAX {
            check_if_in_keyframe(shared.selected_bone().id, shared);
        }
    // scale
    } else if shared.edit_mode == 2 {
        shared.selected_bone_mut().scale = (shared.input.mouse / shared.window) * 2.;
    }
}

/// Get bind group of a texture.
pub fn create_texture(
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

/// Generate and return the vertices of a bone
///
/// Accounts for texture size and aspect ratio
fn rect_verts(
    bone: &Bone,
    camera: &Vec2,
    zoom: f32,
    tex: &Texture,
    aspect_ratio: f32,
) -> Vec<Vertex> {
    let hard_scale = 0.001;
    let mut vertices: Vec<Vertex> = vec![
        Vertex {
            pos: tex.size * bone.scale * hard_scale,
            uv: Vec2::new(1., 0.),
        },
        Vertex {
            pos: tex.size * bone.scale * -hard_scale,
            uv: Vec2::new(0., 1.),
        },
        Vertex {
            pos: Vec2::new(
                -hard_scale * tex.size.x * bone.scale.x,
                hard_scale * tex.size.y * bone.scale.y,
            ),
            uv: Vec2::new(0., 0.),
        },
        Vertex {
            pos: Vec2::new(
                hard_scale * tex.size.x * bone.scale.x,
                -hard_scale * tex.size.y * bone.scale.y,
            ),
            uv: Vec2::new(1., 1.),
        },
    ];

    for v in &mut vertices {
        // rotate verts
        v.pos = utils::rotate(&v.pos, bone.rot);

        // move verts with bone
        v.pos += bone.pos;

        // offset bone with camera
        v.pos -= *camera;

        // adjust for zoom level
        v.pos /= zoom;

        // adjust verts according to aspect ratio
        v.pos.x /= aspect_ratio;
    }

    vertices
}

fn check_if_in_keyframe(id: i32, shared: &mut Shared) {
    let frame = shared.ui.anim.selected_frame;
    // check if this keyframe exists
    let kf = shared
        .selected_animation()
        .keyframes
        .iter()
        .position(|k| k.frame == frame);

    if kf == None {
        // create new keyframe
        shared.selected_animation_mut().keyframes.push(crate::Keyframe {
            frame,
            bones: vec![AnimBone {
                id,
                ..Default::default()
            }],
            ..Default::default()
        });
    } else {
        // check if this bone is in keyframe
        let idx = shared.selected_animation().keyframes[kf.unwrap()]
            .bones
            .iter()
            .position(|bone| bone.id == id);

        if idx == None {
            // create anim bone
            shared.selected_animation_mut().keyframes[kf.unwrap()]
                .bones
                .push(AnimBone {
                    id,
                    ..Default::default()
                });
        }
    }
}
