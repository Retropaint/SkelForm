//! Core rendering logic, abstracted from the rest of WGPU.

use std::f32::consts::PI;

use crate::{
    input,
    shared::{Bone, Shared, Texture, Vec2, Vertex},
    utils, AnimBone,
};
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};
use winit::{keyboard::KeyCode, window::CursorIcon};

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    let mut verts = vec![];

    // for rendering purposes, bones need to have many of their attributes manipulated
    // this is easier to do with a separate copy of them
    let mut temp_bones: Vec<Bone> = vec![];
    for b in &mut shared.armature.bones {
        temp_bones.push(b.clone());
    }

    let mut i = 0;
    // using while loop to prevent borrow errors
    while i < temp_bones.len() {
        macro_rules! bone {
            () => {
                temp_bones[i]
            };
        }

        if bone!().tex_idx == usize::MAX {
            i += 1;
            continue;
        }

        // get parent bone
        let mut p = Bone::default();
        p.scale.x = 1.;
        p.scale.y = 1.;
        if let Some(pp) = find_bone(&temp_bones, bone!().parent_id) {
            p = pp.clone();
        }

        bone!().rot += p.rot;
        bone!().scale *= p.scale;

        // adjust bone's position based on parent's scale
        bone!().pos *= p.scale;

        // rotate such that it will orbit the parent once it's position is inherited
        bone!().pos = utils::rotate(&bone!().pos, p.rot);

        // inherit position from parent
        bone!().pos += p.pos;

        // generate the vertices to be used later
        let this_verts = rect_verts(
            &bone!(),
            &shared.camera.pos,
            shared.zoom,
            &shared.armature.textures[bone!().tex_idx],
            shared.window.x / shared.window.y,
        );
        verts.push(this_verts);

        i += 1;
    }

    let mut hovered_bone = -1;
    let mut hovered_bone_verts: Vec<Vertex> = vec![];

    // Check for bone being hovered on.
    // This has to be in reverse (for now) since bones are rendered in ascending order of the array,
    // so it visually makes sense to click the one that shows in front
    while i > 0 {
        i -= 1;
        if shared.armature.bones[i].tex_idx == usize::MAX {
            continue;
        }
        let is_in_box: bool;
        (hovered_bone_verts, is_in_box) =
            utils::in_bounding_box(&shared.input.mouse, &verts[i], &shared.window);
        if is_in_box {
            // highlight bone for selection if not already selected
            if shared.selected_bone != i {
                hovered_bone = i as i32;

                // select if left clicked
                if shared.input.mouse_left == 0 {
                    shared.selected_bone = i;
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
            edit_bone_with_mouse(shared);
        } else if shared.input.mouse_left != -1 && shared.selected_bone != usize::MAX {
            if shared.input.mouse_left == -1 {
                shared.input.initial_mouse = None;
            } else {
                // move camera if holding mod key
                if let Some(im) = shared.input.initial_mouse {
                    let mouse_world =
                        utils::screen_to_world_space(shared.input.mouse, shared.window);
                    let initial_world = utils::screen_to_world_space(im, shared.window);
                    shared.camera.pos =
                        shared.camera.initial_pos - (mouse_world - initial_world) * shared.zoom;
                } else {
                    shared.camera.initial_pos = shared.camera.pos;
                    shared.input.initial_mouse = Some(shared.input.mouse);
                }
            }
        }
    }
}

pub fn edit_bone_with_mouse(shared: &mut Shared) {
    let mut mouse_world = utils::screen_to_world_space(shared.input.mouse, shared.window);
    mouse_world.x *= shared.window.x / shared.window.y;

    macro_rules! bone {
        () => {
            shared.armature.bones[shared.selected_bone]
        };
    }

    // translation
    if shared.edit_mode == 0 {
        if let Some(parent) = find_bone(&shared.armature.bones, bone!().parent_id) {
            // counteract bone's rotation caused by parent,
            // so that the translation is global
            mouse_world = utils::rotate(&mouse_world, -parent.rot);
        }
        if let Some(offset) = shared.input.initial_mouse {
            // move bone with mouse, keeping in mind their distance
            bone!().pos = (mouse_world * shared.zoom) + offset;

            // record to keyframe if in proper animation context
            if shared.animating && shared.ui.anim.selected != usize::MAX {
                record_to_keyframe(&bone!().clone(), shared);
            }
            shared.cursor_icon = CursorIcon::Move;
        } else {
            // get initial distance between bone and cursor,
            // so that the bone can 'follow' it
            shared.input.initial_mouse = Some(bone!().pos - (mouse_world * shared.zoom));
        }
    // rotation
    } else if shared.edit_mode == 1 {
        bone!().rot = (shared.input.mouse.x / shared.window.x) * PI * 2.;
    } else if shared.edit_mode == 2 {
        bone!().scale = (shared.input.mouse / shared.window) * 2.;
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

        v.pos /= zoom;

        // adjust verts according to aspect ratio
        v.pos.x /= aspect_ratio;
    }

    vertices
}

fn record_to_keyframe(bone: &Bone, shared: &mut Shared) {
    macro_rules! anim {
        () => {
            shared.armature.animations[shared.ui.anim.selected as usize]
        };
    }

    let kf = anim!()
        .keyframes
        .iter()
        .position(|k| k.frame == shared.ui.anim.selected_frame);

    if kf == None {
        // create new keyframe
        anim!().keyframes.push(crate::Keyframe {
            frame: shared.ui.anim.selected_frame,
            bones: vec![AnimBone {
                id: bone.id,
                ..Default::default()
            }],
        });
    } else {
        // check if this bone is in keyframe
        let mut idx = anim!().keyframes[kf.unwrap()]
            .bones
            .iter()
            .position(|bone| bone.id == bone.id);

        if idx == None {
            // create anim bone
            anim!().keyframes[kf.unwrap()].bones.push(AnimBone {
                id: bone.id,
                ..Default::default()
            });
            idx = Some(anim!().keyframes[kf.unwrap()].bones.len());
        }

        // record position into keyframe
        anim!().keyframes[kf.unwrap()].bones[idx.unwrap()].pos = bone.pos;
    }
}

pub fn find_bone(bones: &Vec<Bone>, id: i32) -> Option<&Bone> {
    for b in bones {
        if b.id == id {
            return Some(&b);
        }
    }
    None
}
