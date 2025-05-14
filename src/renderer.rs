//! Core rendering logic, abstracted from the rest of WGPU.

use crate::*;
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};
use winit::keyboard::KeyCode;

macro_rules! con_vert {
    ($func:expr, $vert:expr, $bone:expr, $tex:expr, $shared:expr) => {
        $func(
            $vert,
            Some(&$bone),
            &$shared.camera.pos,
            $shared.camera.zoom,
            Some(&$tex),
            1.,
            0.005,
        )
    };
}

pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    let mut bones = shared.armature.bones.clone();
    if shared.is_animating() {
        bones = shared.animate(shared.ui.anim.selected);
    }

    // For rendering purposes, bones need to have many of their attributes manipulated.
    // This is easier to do with a separate copy of them.
    let mut temp_bones: Vec<Bone> = vec![];
    for b in &mut bones {
        temp_bones.push(b.clone());
    }

    // drawing gridlines
    if shared.generic_bindgroup != None {
        draw_gridline(render_pass, device, shared);
    }

    for i in 0..temp_bones.len() {
        let mut parent: Option<Bone> = None;
        for b in &bones {
            if b.id == temp_bones[i].parent_id {
                parent = Some(b.clone());
                break;
            }
        }

        if parent != None {
            temp_bones[i] = inherit_from_parent(temp_bones[i].clone(), parent.as_ref().unwrap());
        }

        if temp_bones[i].tex_idx != -1 && temp_bones[i].vertices.len() == 0 {
            let tex = &shared.armature.textures[temp_bones[i].tex_idx as usize];
            let temp_verts = create_tex_rect(tex, temp_bones[i].scale);
            shared.armature.bones[i].vertices = temp_verts.clone();
            temp_bones[i].vertices = temp_verts.clone();
        }
    }

    // sort bones by z-index for drawing
    temp_bones.sort_by(|a, b| a.zindex.total_cmp(&b.zindex));

    // draw bone
    for b in 0..temp_bones.len() {
        draw_bone(&temp_bones[b], shared, render_pass, device);
        render_pass.set_bind_group(0, &shared.generic_bindgroup, &[]);
        draw_point(
            &Vec2::ZERO,
            &shared,
            render_pass,
            device,
            &temp_bones[b],
            Color::GREEN,
            shared.camera.pos,
        );
        if temp_bones[b].is_mesh {
            bone_vertices(&temp_bones[b], shared, render_pass, device);
        }
    }

    if shared.input.mouse_left == -1 {
        shared.dragging_vert = usize::MAX;
    } else if shared.dragging_vert != usize::MAX {
        drag_vertex(shared, shared.dragging_vert);
        return;
    }

    // if mouse_left is lower than this, it's considered a click
    let click_threshold = 10;

    if shared.input.mouse_left == -1 {
        shared.editing_bone = false;

        if shared.selected_bone() != None
            && shared.input.mouse_left_prev != -1
            && shared.input.mouse_left_prev < click_threshold
            && shared.selected_bone().unwrap().is_mesh
        {
            let bone = &shared.selected_bone().unwrap();
            let tex = &shared.armature.textures[bone.tex_idx as usize];

            // create vert on cursor
            let vert = world_to_raw_vert(
                Vertex {
                    pos: utils::screen_to_world_space(shared.input.mouse, shared.window),
                    ..Default::default()
                },
                Some(&bone),
                &shared.camera.pos,
                shared.camera.zoom,
                Some(&tex),
                1.,
                0.005,
            );

            let world_verts: Vec<Vertex> = vec![];
            for vert in &bone.vertices {}

            // find nearest 2 verts to connect to
            shared.selected_bone_mut().unwrap().vertices.push(vert);
        }
        return;
    }

    // mouse related stuff

    // move camera
    if shared.input.is_pressing(KeyCode::SuperLeft) || shared.selected_bone_idx == usize::MAX {
        shared.camera.pos -= shared.mouse_vel() * shared.camera.zoom;
        return;
    }

    // editing bone
    if shared.input.on_ui || shared.ui.has_state(UiState::PolarModal) {
        shared.editing_bone = false;
    } else if shared.selected_bone_idx != usize::MAX && shared.input.mouse_left > click_threshold {
        // save animation for undo
        if !shared.editing_bone {
            shared.save_edited_bone();
            shared.editing_bone = true;
        }

        shared.cursor_icon = egui::CursorIcon::Crosshair;

        let bone = &bones[shared.selected_bone_idx];
        edit_bone(shared, bone);
    }
}

pub struct Triangle {
    pub verts: [Vertex; 3],
    pub idx: [usize; 3],
}

pub fn edit_bone(shared: &mut Shared, bone: &Bone) {
    match shared.edit_mode {
        shared::EditMode::Move => {
            let mut pos = Vec2::default();
            if !shared.is_animating() {
                pos = bone.pos;
            } else {
                // get animated position
                for kf in &shared.selected_animation().unwrap().keyframes {
                    if kf.bone_id == bone.id && kf.element == AnimElement::PositionX {
                        pos.x = kf.value;
                    }
                    if kf.bone_id == bone.id && kf.element == AnimElement::PositionY {
                        pos.y = kf.value;
                    }
                }
            }

            // offset position by said velocity
            pos += shared.mouse_vel() * shared.camera.zoom;

            shared.edit_bone(&AnimElement::PositionX, pos.x, false);
            shared.edit_bone(&AnimElement::PositionY, pos.y, false);
        }
        shared::EditMode::Rotate => {
            let rot = (shared.input.mouse.x / shared.window.x) * std::f32::consts::PI * 2.;
            shared.edit_bone(&AnimElement::Rotation, rot, false);
        }
        shared::EditMode::Scale => {
            let scale = (shared.input.mouse / shared.window) * 2.;
            shared.edit_bone(&AnimElement::ScaleX, scale.x, false);
            shared.edit_bone(&AnimElement::ScaleY, scale.y, false);
        }
    };
}

pub fn inherit_from_parent(mut child: Bone, parent: &Bone) -> Bone {
    child.rot += parent.rot;
    child.scale *= parent.scale;

    // adjust bone's position based on parent's scale
    child.pos *= parent.scale;

    // rotate such that it will orbit the parent once it's position is inherited
    child.pos = utils::rotate(&child.pos, parent.rot);

    // inherit position from parent
    child.pos += parent.pos;

    child
}

pub fn draw_bone(bone: &Bone, shared: &Shared, render_pass: &mut RenderPass, device: &Device) {
    if bone.tex_idx == -1 {
        return;
    }

    render_pass.set_bind_group(0, &shared.bind_groups[bone.tex_idx as usize], &[]);

    for v in 0..bone.vertices.len() {
        if v > bone.vertices.len() - 3 {
            break;
        }

        macro_rules! world_vert {
            ($idx:expr) => {
                raw_to_world_vert(
                    bone.vertices[$idx],
                    Some(&bone),
                    &shared.camera.pos,
                    shared.camera.zoom,
                    Some(&shared.armature.textures[bone.tex_idx as usize]),
                    shared.window.x / shared.window.y,
                    0.005,
                )
            };
        }

        let verts = vec![world_vert!(v), world_vert!(v + 1), world_vert!(v + 2)];
        render_pass.set_vertex_buffer(0, vertex_buffer(&verts, device).slice(..));

        render_pass.set_index_buffer(
            index_buffer(vec![0, 1, 2], &device).slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..3, 0, 0..1);
    }
}

pub fn bone_vertices(
    bone: &Bone,
    shared: &mut Shared,
    render_pass: &mut RenderPass,
    device: &Device,
) {
    for v in 0..bone.vertices.len() {
        if v > bone.vertices.len() - 3 {
            break;
        }

        macro_rules! world_vert {
            ($idx:expr) => {
                raw_to_world_vert(
                    bone.vertices[$idx],
                    Some(&bone),
                    &shared.camera.pos,
                    shared.camera.zoom,
                    Some(&shared.armature.textures[bone.tex_idx as usize]),
                    shared.window.x / shared.window.y,
                    0.005,
                )
            };
        }

        let verts = vec![world_vert!(v), world_vert!(v + 1), world_vert!(v + 2)];
        macro_rules! point {
            ($idx:expr, $color:expr) => {
                draw_point(
                    &verts[$idx].pos,
                    &shared,
                    render_pass,
                    device,
                    &Bone {
                        pos: Vec2::ZERO,
                        ..bone.clone()
                    },
                    $color,
                    Vec2::ZERO,
                )
            };
        }

        for wv in 0..verts.len() {
            let point = point!(wv, Color::GREEN);
            let clicking_on_it =
                utils::in_bounding_box(&shared.input.mouse, &point, &shared.window).1;
            if clicking_on_it && shared.dragging_vert == usize::MAX {
                point!(wv, Color::WHITE);

                if shared.input.mouse_left != -1 {
                    shared.dragging_vert = v + wv;
                    continue;
                }
            }
        }
    }
}

pub fn drag_vertex(shared: &mut Shared, vert_idx: usize) {
    let bone = shared.selected_bone().unwrap();
    let tex = &shared.armature.textures[bone.tex_idx as usize];

    let mut world_vert = con_vert!(
        raw_to_world_vert,
        bone.vertices[vert_idx],
        bone,
        tex,
        shared
    );
    world_vert.pos = utils::screen_to_world_space(shared.input.mouse, shared.window);
    shared.selected_bone_mut().unwrap().vertices[vert_idx].pos =
        con_vert!(world_to_raw_vert, world_vert, bone, tex, shared).pos;
}

pub fn create_tex_rect(tex: &Texture, scale: Vec2) -> Vec<Vertex> {
    vec![
        Vertex::default(),
        Vertex {
            pos: Vec2::new(tex.size.x * scale.x, 0.),
            uv: Vec2::new(1., 0.),
            color: Color::default(),
        },
        Vertex {
            pos: Vec2::new(0., tex.size.y * -scale.y),
            uv: Vec2::new(0., 1.),
            color: Color::default(),
        },
        Vertex {
            pos: Vec2::new(tex.size.x * scale.x, tex.size.y * -scale.y),
            uv: Vec2::new(1., 1.),
            color: Color::default(),
        },
    ]
}

fn draw_point(
    offset: &Vec2,
    shared: &Shared,
    render_pass: &mut RenderPass,
    device: &Device,
    bone: &Bone,
    color: Color,
    camera: Vec2,
) -> Vec<Vertex> {
    if shared.generic_bindgroup == None {
        return vec![];
    }

    let point_size = 0.1;
    let temp_point_verts: [Vertex; 4] = [
        Vertex {
            pos: Vec2::new(-point_size, point_size) + bone.pos,
            uv: Vec2::new(1., 0.),
            color,
        },
        Vertex {
            pos: Vec2::new(point_size, point_size) + bone.pos,
            uv: Vec2::new(0., 1.),
            color,
        },
        Vertex {
            pos: Vec2::new(-point_size, -point_size) + bone.pos,
            uv: Vec2::new(0., 0.),
            color,
        },
        Vertex {
            pos: Vec2::new(point_size, -point_size) + bone.pos,
            uv: Vec2::new(1., 1.),
            color,
        },
    ];

    let mut point_verts = rect_verts(
        temp_point_verts.to_vec(),
        None,
        &camera,
        shared.camera.zoom,
        None,
        shared.window.x / shared.window.y,
        1.,
    );

    for vert in &mut point_verts {
        vert.pos += *offset;
    }

    render_pass.set_vertex_buffer(0, vertex_buffer(&point_verts.to_vec(), device).slice(..));
    render_pass.set_index_buffer(
        index_buffer(RECT_VERT_INDICES.to_vec(), &device).slice(..),
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

fn rect_verts(
    mut verts: Vec<Vertex>,
    bone: Option<&Bone>,
    camera: &Vec2,
    zoom: f32,
    tex: Option<&Texture>,
    aspect_ratio: f32,
    hard_scale: f32,
) -> Vec<Vertex> {
    for i in 0..verts.len() {
        verts[i] = raw_to_world_vert(verts[i], bone, camera, zoom, tex, aspect_ratio, hard_scale);
    }

    verts.to_vec()
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

    let gap = 2.;
    let width = 0.005 * shared.camera.zoom;
    let regular_color = Color::new(0.5, 0.5, 0.5, 0.25);
    let highlight_color = Color::new(0.7, 0.7, 0.7, 1.);
    let mut color = Color::new(0.5, 0.5, 0.5, 0.25);

    // Used to highlight center horizontal and vertical lines,
    // but also to prevent bind group from being set for every line
    // (causes slowdown)
    let mut center_line = false;

    let aspect_ratio = shared.window.y / shared.window.x;
    let mut x = shared.camera.pos.x - shared.camera.zoom / aspect_ratio;
    x = x.round();
    while x < shared.camera.pos.x + shared.camera.zoom / aspect_ratio {
        if x % gap != 0. {
            x += 1.;
            continue;
        }
        if x == 0. && !center_line {
            color = highlight_color;
            center_line = true;
        } else if x != 0. && center_line {
            color = regular_color;
            center_line = false;
        }
        draw_vertical_line(x, width, render_pass, device, shared, color);
        x += 1.;
    }

    color = Color::new(0.5, 0.5, 0.5, 0.5);

    // reset bind group to regular, non-highlighted one
    center_line = false;

    let mut y = shared.camera.pos.y - shared.camera.zoom;
    y = y.round();
    while y < shared.camera.pos.y + shared.camera.zoom {
        if y % gap != 0. {
            y += 1.;
            continue;
        }
        if y == 0. && !center_line {
            color = highlight_color;
            center_line = true;
        } else if y != 0. && center_line {
            color = regular_color;
            center_line = false;
        }
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
    color: Color,
) {
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: (Vec2::new(-200., y) - shared.camera.pos) / shared.camera.zoom,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(0., width + y) - shared.camera.pos) / shared.camera.zoom,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(200., y) - shared.camera.pos) / shared.camera.zoom,
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
    color: Color,
) {
    let aspect_ratio = shared.window.y / shared.window.x;
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: (Vec2::new(x, -200.) - shared.camera.pos) / shared.camera.zoom * aspect_ratio,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(width + x, 0.) - shared.camera.pos) / shared.camera.zoom * aspect_ratio,
            color,
            ..Default::default()
        },
        Vertex {
            pos: (Vec2::new(x, 200.) - shared.camera.pos) / shared.camera.zoom * aspect_ratio,
            color,
            ..Default::default()
        },
    ];
    render_pass.set_vertex_buffer(0, vertex_buffer(&vertices, device).slice(..));
    render_pass.draw_indexed(0..3, 0, 0..1);
}
