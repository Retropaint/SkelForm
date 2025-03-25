//! Core rendering logic, abstracted from the rest of WGPU.

use crate::{
    shared::{Armature, Bone, Shared},
    utils, vec2, Vec2, Vertex,
};
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    let mut i = 0;
    for b in &mut shared.armature.bones {
        if b.tex_idx == usize::MAX {
            continue;
        }
        if shared.mouse_left < 2 {
            let mouse_world = utils::screen_to_world_space(shared.mouse, shared.window);
            shared.mouse_bone_offset = vec2! {b.pos.x - mouse_world.x, b.pos.y - mouse_world.y};
        }
        if shared.selected_bone == i && shared.mouse_left > 0 {
            let mouse_world = utils::screen_to_world_space(shared.mouse, shared.window);
            b.pos = vec2! { mouse_world.x + shared.mouse_bone_offset.x, mouse_world.y + shared.mouse_bone_offset.y };
        }
        let verts = rect_verts(&b);
        render_pass.set_bind_group(0, &shared.bind_groups[b.tex_idx], &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer(&verts, device).slice(..));
        render_pass.set_index_buffer(
            index_buffer([0, 1, 2, 0, 1, 3].to_vec(), &device).slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
        if utils::in_bounding_box(&shared.mouse, &verts, &shared.window) {}
        i += 1;
    }
}

/// Get bind group of a texture.
pub fn create_texture(
    pixels: Vec<u8>,
    dimensions: Vec2,
    textures: &mut Vec<crate::shared::Texture>,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
) -> BindGroup {
    // add to shared textures
    textures.push(crate::Texture {
        size: dimensions,
        pixels: pixels.to_vec(),
    });

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

fn rect_verts(bone: &Bone) -> Vec<Vertex> {
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: vec2! {0.5 + bone.pos.x, 0.5 + bone.pos.y},
            uv: vec2! {1., 0.},
        },
        Vertex {
            pos: vec2! {-0.5 + bone.pos.x, -0.5 + bone.pos.y},
            uv: vec2! {0., 1.},
        },
        Vertex {
            pos: vec2! {-0.5 + bone.pos.x, 0.5 + bone.pos.y},
            uv: vec2! {0., 0.},
        },
        Vertex {
            pos: vec2! {0.5 + bone.pos.x, -0.5 + bone.pos.y},
            uv: vec2! {1., 1.},
        },
        Vertex {
            pos: vec2! {0.25 + bone.pos.x, -0.25 + bone.pos.y},
            uv: vec2! {1., 1.},
        },
    ];

    vertices
}
