//! Core rendering logic, abstracted from the rest of WGPU

use image::EncodableLayout;

use crate::{shared::Shared, utils::screen_to_world_space, Vec2, Vertex};
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};

// wasm-only imports
#[cfg(target_arch = "wasm32")]
mod wasm {
    pub use crate::utils::load_image_wasm;
}
#[cfg(target_arch = "wasm32")]
use wasm::*;

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use image::GenericImageView;
    pub use image::{DynamicImage, ImageBuffer, ImageResult, Rgba};
    pub use std::fs;
}
#[cfg(not(target_arch = "wasm32"))]
use native::*;

macro_rules! vec2 {
    ($x_var:expr, $y_var:expr) => {
        Vec2 {
            x: $x_var,
            y: $y_var,
        }
    };
}

const VERTICES: [Vertex; 4] = [
    Vertex {
        position: vec2! {0.5, 0.5},
        uv: vec2! {1., 0.},
    },
    Vertex {
        position: vec2! {-0.5, -0.5},
        uv: vec2! {0., 1.},
    },
    Vertex {
        position: vec2! {-0.5, 0.5},
        uv: vec2! {0., 0.},
    },
    Vertex {
        position: vec2! {0.5, -0.5},
        uv: vec2! {1., 1.},
    },
];

const INDICES: [u32; 6] = [0, 1, 2, 0, 1, 3];

/// The `main` of this module
pub fn render(
    render_pass: &mut RenderPass,
    queue: &Queue,
    device: &Device,
    shared: &mut Shared,
    bind_group_layout: &BindGroupLayout,
) {
    // automatic first bind group for testing
    if shared.bind_groups.len() == 0 {
        shared.bind_groups.push(create_texture(
            "./gopher.png",
            queue,
            device,
            &bind_group_layout,
        ));
    }
    render_pass.set_bind_group(0, &shared.bind_groups[0], &[]);

    // set up vertices
    let mut vertices = VERTICES.clone();
    vertices[0].position = screen_to_world_space(shared.mouse, shared.window);
    let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        },
    );
    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

    // set up indices
    let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("index Buffer"),
            contents: bytemuck::cast_slice(&INDICES),
            usage: wgpu::BufferUsages::INDEX,
        },
    );
    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

    // finally, draw!
    render_pass.draw_indexed(0..3, 0, 0..1);
    render_pass.draw_indexed(3..6, 0, 0..1);
}

/// Get bind group of a texture
pub fn create_texture(
    img_path: &str,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
) -> BindGroup {
    #[cfg(not(target_arch = "wasm32"))]
    let diffuse_image: ImageResult<DynamicImage>;
    #[cfg(not(target_arch = "wasm32"))]
    let rgba: ImageBuffer<Rgba<u8>, Vec<u8>>;
    #[cfg(target_arch = "wasm32")]
    let rgba: Vec<u8>;

    let diffuse_rgba: &[u8];
    let dimensions: (u32, u32);

    if img_path == "" {
        // create solid magenta image if path is empty
        dimensions = (1, 1);
        diffuse_rgba = &[255, 0, 255, 255];
    } else {
        // load image via fs & image crate for native
        #[cfg(not(target_arch = "wasm32"))]
        {
            let bytes = fs::read(img_path);
            diffuse_image = Ok(image::load_from_memory(&bytes.unwrap()).unwrap());
            dimensions = diffuse_image.as_ref().unwrap().dimensions();
            rgba = diffuse_image.unwrap().to_rgba8();
            diffuse_rgba = rgba.as_bytes();
        }
        // load image via DOM for WASM
        #[cfg(target_arch = "wasm32")]
        {
            let dims: Vec2;
            (rgba, dims) = load_image_wasm();
            dimensions = (dims.x as u32, dims.y as u32);
            diffuse_rgba = rgba.as_bytes();
        }
    }

    let tex_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        size: tex_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
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
        &diffuse_rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
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
