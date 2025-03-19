/// This is the core rendering logic, abstracted from the rest of WGPU

use std::fs;

use image::{DynamicImage, ImageResult, RgbaImage};
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};

use crate::{utils::screen_to_world_space, Skelements, Vec2, Vertex};

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

pub fn render(
    render_pass: &mut RenderPass,
    queue: &Queue,
    device: &Device,
    skelements: &Skelements,
    bind_group_layout: &BindGroupLayout,
) {
    let bind_group = create_texture(queue, device, "./gopher.png", &bind_group_layout);

    render_pass.set_bind_group(0, &bind_group, &[]);

    let mut vertices = VERTICES.clone();
    vertices[0].position = screen_to_world_space(skelements.mouse, skelements.window);
    let vertex_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        },
    );

    let index_buffer = wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label: Some("index Buffer"),
            contents: bytemuck::cast_slice(&INDICES),
            usage: wgpu::BufferUsages::INDEX,
        },
    );

    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

    render_pass.draw_indexed(0..3, 0, 0..1);
    render_pass.draw_indexed(3..6, 0, 0..1);
}

fn create_texture(
    queue: &Queue,
    device: &Device,
    img_path: &str,
    bind_group_layout: &BindGroupLayout,
) -> BindGroup {
    let diffuse_image: ImageResult<DynamicImage>;
    let diffuse_rgba: RgbaImage;
    let dimensions: (u32, u32);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let bytes = fs::read(img_path);
        diffuse_image = Ok(image::load_from_memory(&bytes.unwrap()).unwrap());
        dimensions = diffuse_image.as_ref().unwrap().dimensions();
        diffuse_rgba = diffuse_image.unwrap().to_rgba8();
    }

    use image::GenericImageView;

    let tex_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        size: tex_size,
        mip_level_count: 1, // We'll talk about this a little later
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("diffuse_texture"),
        view_formats: &[],
    });
    queue.write_texture(
        // Tells wgpu where to copy the pixel data
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        // The actual pixel data
        &diffuse_rgba,
        // The layout of the texture
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
