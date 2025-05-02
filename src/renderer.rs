//! Core rendering logic, abstracted from the rest of WGPU.

use crate::{
    shared::{Bone, Shared, Texture, Vec2, Vertex},
    utils, RECT_VERT_INDICES,
};
use wgpu::{BindGroup, BindGroupLayout, Device, Queue, RenderPass};
use winit::keyboard::KeyCode;

/// The `main` of this module.
pub fn render(render_pass: &mut RenderPass, device: &Device, shared: &mut Shared) {
    let mut bones = shared.armature.bones.clone();
    if shared.is_animating() {
        bones = shared.animate(shared.ui.anim.selected);
    }

    let mut verts = vec![];

    // For rendering purposes, bones need to have many of their attributes manipulated.
    // This is easier to do with a separate copy of them.
    let mut temp_bones: Vec<Bone> = vec![];
    for b in &mut bones {
        temp_bones.push(b.clone());
    }

    // drawing gridlines
    if shared.gridline_bindgroup != None {
        render_pass.set_index_buffer(
            index_buffer([0, 1, 2].to_vec(), &device).slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_bind_group(0, &shared.gridline_bindgroup, &[]);

        let gap = 2.;

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
                render_pass.set_bind_group(0, &shared.highlight_bindgroup, &[]);
                center_line = true;
            } else if x != 0. && center_line {
                render_pass.set_bind_group(0, &shared.gridline_bindgroup, &[]);
                center_line = false;
            }
            draw_vertical_line(x, 0.005 * shared.camera.zoom, render_pass, device, shared);
            x += 1.;
        }

        center_line = false;
        let mut y = shared.camera.pos.y - shared.camera.zoom;
        y = y.round();
        while y < shared.camera.pos.y + shared.camera.zoom {
            if y % gap != 0. {
                y += 1.;
                continue;
            }
            if y == 0. && !center_line {
                render_pass.set_bind_group(0, &shared.highlight_bindgroup, &[]);
                center_line = true;
            } else if y != 0. && center_line {
                render_pass.set_bind_group(0, &shared.gridline_bindgroup, &[]);
                center_line = false;
            }
            draw_horizontal_line(y, 0.005 * shared.camera.zoom, render_pass, device, shared);
            y += 1.;
        }
    }

    // using while loop to prevent borrow issues
    for i in 0..temp_bones.len() {
        if temp_bones[i].tex_idx == -1 {
            verts.push(vec![]);
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

        let tex = &shared.armature.textures[temp_bones[i].tex_idx as usize];

        let temp_verts: [Vertex; 4] = [
            Vertex {
                pos: tex.size * temp_bones[i].scale,
                uv: Vec2::new(1., 0.),
            },
            Vertex {
                pos: tex.size * temp_bones[i].scale * -1.,
                uv: Vec2::new(0., 1.),
            },
            Vertex {
                pos: Vec2::new(
                    tex.size.x * temp_bones[i].scale.x * -1.,
                    tex.size.y * temp_bones[i].scale.y,
                ),
                uv: Vec2::new(0., 0.),
            },
            Vertex {
                pos: Vec2::new(
                    tex.size.x * temp_bones[i].scale.x,
                    tex.size.y * temp_bones[i].scale.y * -1.,
                ),
                uv: Vec2::new(1., 1.),
            },
        ];

        // generate the vertices to be used later
        let final_verts = rect_verts(
            temp_verts,
            Some(&temp_bones[i]),
            &shared.camera.pos,
            shared.camera.zoom,
            Some(&shared.armature.textures[temp_bones[i].tex_idx as usize]),
            shared.window.x / shared.window.y,
            0.005,
        );

        verts.push(final_verts.clone());
        shared.armature.bones[i].vertices = final_verts;
    }

    let mut hovered_bone = -1;
    let mut hovered_bone_verts: Vec<Vertex> = vec![];
    let can_hover = !shared.input.on_ui
        && shared.ui.polar_id == ""
        && !shared.ui.image_modal
        && !shared.editing_bone;

    // Check for the bone being hovered on.
    // This has to be in reverse (for now) since bones are rendered in ascending order of the array,
    // so it visually makes sense to click the one that shows in front.
    if can_hover {
        for i in (0..temp_bones.len()).rev() {
            if shared.armature.bones[i].tex_idx == -1 || verts[i].len() == 0 {
                continue;
            }

            // Check if this bone is a child of the selected bone.
            // If so, ignore.
            if shared.selected_bone() != None {
                let mut ignore = false;
                let mut parent = shared.find_bone(temp_bones[i].parent_id);
                while parent != None {
                    if parent.unwrap().id == shared.selected_bone().unwrap().id {
                        ignore = true;
                        break;
                    }
                    parent = shared.find_bone(parent.unwrap().parent_id);
                }
                if ignore {
                    continue;
                }
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
                        shared.select_bone(i as usize);
                    }
                }
                break;
            }
        }
    }

    // finally, draw the bones
    for (i, b) in temp_bones.iter().enumerate() {
        if b.tex_idx == -1 || verts[i].len() == 0 {
            if shared.selected_bone_idx == i {
                draw_point(shared, render_pass, device, b);
            }
            continue;
        }

        // draw the hovering highlight section
        if hovered_bone as usize == i && can_hover {
            render_pass.set_bind_group(0, &shared.highlight_bindgroup, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer(&hovered_bone_verts, device).slice(..));
            render_pass.set_index_buffer(
                index_buffer([0, 1, 2, 3, 0, 1].to_vec(), &device).slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        // draw bone
        render_pass.set_bind_group(0, &shared.bind_groups[b.tex_idx as usize], &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer(&verts[i], device).slice(..));
        render_pass.set_index_buffer(
            index_buffer(RECT_VERT_INDICES.to_vec(), &device).slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);

        if shared.selected_bone_idx == i {
            draw_point(shared, render_pass, device, b);
        }
    }

    // if mouse_left is lower than this, it's considered a click
    let click_threshold = 10;

    if shared.input.mouse_left == -1 {
        // deselect bone if clicking outside
        if hovered_bone == -1
            && shared.input.mouse_left_prev <= click_threshold
            && shared.input.mouse_left_prev != -1
            && can_hover
        {
            shared.selected_bone_idx = usize::MAX;
        }

        shared.editing_bone = false;
        return;
    }

    // mouse related stuff

    // move camera
    if shared.input.is_pressing(KeyCode::SuperLeft) || shared.selected_bone_idx == usize::MAX {
        if shared.input.initial_points.len() == 0 {
            shared.camera.initial_pos = shared.camera.pos;
            shared.input.initial_points.push(shared.input.mouse);
        }

        let mouse_world = utils::screen_to_world_space(shared.input.mouse, shared.window);
        let initial_world =
            utils::screen_to_world_space(shared.input.initial_points[0], shared.window);
        shared.camera.pos =
            shared.camera.initial_pos - (mouse_world - initial_world) * shared.camera.zoom;

        return;
    }

    // editing bone
    if shared.input.on_ui || shared.ui.polar_id != "" {
        shared.editing_bone = false;
    } else if shared.selected_bone_idx != usize::MAX && shared.input.mouse_left > click_threshold {
        if !shared.editing_bone {
            if shared.is_animating() {
                shared.undo_actions.push(crate::Action {
                    action: crate::ActionEnum::Animation,
                    action_type: crate::ActionType::Edited,
                    id: shared.ui.anim.selected as i32,
                    animation: shared.armature.animations[shared.ui.anim.selected].clone(),
                    ..Default::default()
                });
            } else {
                shared.save_edited_bone();
            }
        }

        shared.editing_bone = true;
        shared.cursor_icon = egui::CursorIcon::Crosshair;
        let value: Vec2;
        value = match shared.edit_mode {
            // translation
            0 => {
                let mut pos = bones[shared.selected_bone_idx].pos;
                if shared.is_animating() {
                    pos = bones[shared.selected_bone_idx].pos
                        - shared.armature.bones[shared.selected_bone_idx].pos;
                }
                shared.move_with_mouse(&pos, true)
            }

            // rotation
            1 => Vec2::single((shared.input.mouse.x / shared.window.x) * std::f32::consts::PI * 2.),

            // scale
            2 => (shared.input.mouse / shared.window) * 2.,

            _ => Vec2::default(),
        };
        shared.edit_bone(shared.edit_mode, value, false);
    }
}

fn draw_point(shared: &Shared, render_pass: &mut RenderPass, device: &Device, bone: &Bone) {
    if shared.point_bindgroup != None {
        render_pass.set_bind_group(0, &shared.point_bindgroup, &[]);
        let point_size = 0.1;
        let temp_point_verts: [Vertex; 4] = [
            Vertex {
                pos: Vec2::new(-point_size, point_size) + bone.pos,
                uv: Vec2::new(1., 0.),
            },
            Vertex {
                pos: Vec2::new(point_size, point_size) + bone.pos,
                uv: Vec2::new(0., 1.),
            },
            Vertex {
                pos: Vec2::new(-point_size, -point_size) + bone.pos,
                uv: Vec2::new(0., 0.),
            },
            Vertex {
                pos: Vec2::new(point_size, -point_size) + bone.pos,
                uv: Vec2::new(1., 1.),
            },
        ];

        let point_verts = rect_verts(
            temp_point_verts,
            None,
            &shared.camera.pos,
            shared.camera.zoom,
            None,
            shared.window.x / shared.window.y,
            1.,
        );

        render_pass.set_vertex_buffer(0, vertex_buffer(&point_verts.to_vec(), device).slice(..));
        render_pass.set_index_buffer(
            index_buffer(RECT_VERT_INDICES.to_vec(), &device).slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
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

/// Generate and return the vertices of a bone
///
/// Accounts for texture size and aspect ratio
fn rect_verts(
    mut verts: [Vertex; 4],
    bone: Option<&Bone>,
    camera: &Vec2,
    zoom: f32,
    tex: Option<&Texture>,
    aspect_ratio: f32,
    hard_scale: f32,
) -> Vec<Vertex> {
    for v in &mut verts {
        v.pos = v.pos * hard_scale;

        if let Some(bone) = bone {
            let pivot_offset = tex.unwrap().size * bone.pivot * hard_scale;
            v.pos.x -= pivot_offset.x;
            v.pos.y += pivot_offset.y;

            let rev_scale = Vec2::new(1. - bone.scale.x, 1. - bone.scale.y);
            let scale_offset = tex.unwrap().size * rev_scale * bone.pivot * hard_scale;
            v.pos.x += scale_offset.x;
            v.pos.y -= scale_offset.y;

            // rotate verts
            v.pos = utils::rotate(&v.pos, bone.rot);

            // move verts with bone
            v.pos += bone.pos;
        }

        // offset bone with camera
        v.pos -= *camera;

        // adjust for zoom level
        v.pos /= zoom;

        // adjust verts for aspect ratio
        v.pos.x /= aspect_ratio;
    }

    verts.to_vec()
}

pub fn draw_horizontal_line(
    y: f32,
    width: f32,
    render_pass: &mut RenderPass,
    device: &Device,
    shared: &Shared,
) {
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: (Vec2::new(-200., y) - shared.camera.pos) / shared.camera.zoom,
            uv: Vec2::ZERO,
        },
        Vertex {
            pos: (Vec2::new(0., width + y) - shared.camera.pos) / shared.camera.zoom,
            uv: Vec2::ZERO,
        },
        Vertex {
            pos: (Vec2::new(200., y) - shared.camera.pos) / shared.camera.zoom,
            uv: Vec2::ZERO,
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
) {
    let aspect_ratio = shared.window.y / shared.window.x;
    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: (Vec2::new(x, -200.) - shared.camera.pos) / shared.camera.zoom * aspect_ratio,
            uv: Vec2::ZERO,
        },
        Vertex {
            pos: (Vec2::new(width + x, 0.) - shared.camera.pos) / shared.camera.zoom * aspect_ratio,
            uv: Vec2::ZERO,
        },
        Vertex {
            pos: (Vec2::new(x, 200.) - shared.camera.pos) / shared.camera.zoom * aspect_ratio,
            uv: Vec2::ZERO,
        },
    ];
    render_pass.set_vertex_buffer(0, vertex_buffer(&vertices, device).slice(..));
    render_pass.draw_indexed(0..3, 0, 0..1);
}
