//! Isolated set of helper functions.

use crate::*;

#[cfg(target_arch = "wasm32")]
mod web {
    pub use wasm_bindgen::prelude::wasm_bindgen;
    pub use web_sys::*;
    pub use zip::write::FileOptions;
}
#[cfg(target_arch = "wasm32")]
pub use web::*;

use image::ImageEncoder;

use std::io::{Read, Write};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn downloadZip(zip: Vec<u8>);
}

/// Convert a point from screen to world space.
pub fn screen_to_world_space(pos: Vec2, window: Vec2) -> Vec2 {
    let aspect_ratio = window.x / window.y;
    Vec2 {
        x: (-1. + (pos.x / window.x as f32 * 2.)) * aspect_ratio,
        y: -(-1. + (pos.y / window.y as f32 * 2.)),
    }
}

pub fn world_to_screen_space(pos: Vec2, window: Vec2, zoom: f32, use_aspect_ratio: bool) -> Vec2 {
    let mut aspect_ratio = window.y / window.x;
    if !use_aspect_ratio {
        aspect_ratio = 1.;
    }

    let mut vec2 = Vec2::new(
        (pos.x * window.x as f32 / 4.) * aspect_ratio,
        -(pos.y * window.y as f32 / 4.),
    );
    vec2 /= zoom;
    vec2 += window / 4.;

    vec2
}

/// Rotate a point via rotation matrix.
pub fn rotate(point: &Vec2, rot: f32) -> Vec2 {
    Vec2 {
        x: point.x * rot.cos() - point.y * rot.sin(),
        y: point.x * rot.sin() + point.y * rot.cos(),
    }
}

/// Return the angle that the source would need to look at target.
pub fn look_at(source: &Vec2, target: &Vec2) -> f32 {
    f32::atan2(-(target.x - source.x), target.y - source.y)
}

/// Check if a point is in a rectangle (formed by vertices).
pub fn in_bounding_box(
    point: &Vec2,
    verts: &Vec<Vertex>,
    window_size: &Vec2,
) -> (Vec<Vertex>, bool) {
    // get the bound based on infinitely-long lines
    let mut top = -f32::INFINITY;
    let mut bot = f32::INFINITY;
    let mut left = f32::INFINITY;
    let mut right = -f32::INFINITY;
    for v in verts {
        left = f32::min(left, v.pos.x);
        right = f32::max(right, v.pos.x);
        bot = f32::min(bot, v.pos.y);
        top = f32::max(top, v.pos.y);
    }

    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: Vec2::new(right, top),
            uv: Vec2::new(1., 0.),
            color: Color::default(),
        },
        Vertex {
            pos: Vec2::new(left, top),
            uv: Vec2::new(0., 1.),
            color: Color::default(),
        },
        Vertex {
            pos: Vec2::new(left, bot),
            uv: Vec2::new(0., 0.),
            color: Color::default(),
        },
        Vertex {
            pos: Vec2::new(right, bot),
            uv: Vec2::new(1., 1.),
            color: Color::default(),
        },
    ];

    // convert bound positions to screen space
    let half = Vec2 {
        x: window_size.x / 2.,
        y: window_size.y / 2.,
    };
    top = half.y - (half.y * top);
    bot = half.y - (half.y * bot);
    left = half.x + (half.x * left);
    right = half.x + (half.x * right);

    // finally, check if point is inside
    (
        vertices,
        point.y > top && point.y < bot && point.x > left && point.x < right,
    )
}

pub fn to_vec2(f: f32) -> Vec2 {
    Vec2::new(f, f)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_save_dialog() {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        let task = rfd::FileDialog::new().save_file();
        if task == None {
            return;
        }
        file_reader::create_temp_file(TEMP_SAVE_PATH, task.unwrap().as_path().to_str().unwrap());
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_import_dialog() {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || {
        let task = rfd::FileDialog::new().pick_file();
        if task == None {
            return;
        }
        file_reader::create_temp_file(TEMP_IMPORT_PATH, task.unwrap().as_path().to_str().unwrap());
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(path: String, shared: &mut Shared) {
    let (size, armatures_json, png_buf) = prepare_files(shared);

    // create zip file
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // save armature json and texture image
    zip.start_file("armature.json", options).unwrap();
    zip.write(armatures_json.as_bytes()).unwrap();
    if size != Vec2::ZERO {
        zip.start_file("textures.png", options).unwrap();
        zip.write(&png_buf).unwrap();
    }

    zip.finish().unwrap();
}

#[cfg(target_arch = "wasm32")]
pub fn save_web(shared: &mut Shared) {
    let (size, armatures_json, png_buf) = prepare_files(shared);

    // create zip file
    let mut buf: Vec<u8> = Vec::new();
    let cursor = std::io::Cursor::new(&mut buf);
    let mut zip = zip::ZipWriter::new(cursor);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // save armature json and texture image
    zip.start_file("armature.json", options).unwrap();
    zip.write(armatures_json.as_bytes()).unwrap();
    if size != Vec2::ZERO {
        zip.start_file("textures.png", options).unwrap();
        zip.write(&png_buf).unwrap();
    }

    let bytes = zip.finish().unwrap().into_inner().to_vec();
    downloadZip(bytes);
}

fn create_tex_sheet(shared: &mut Shared, size: &Vec2) -> std::vec::Vec<u8> {
    // set up the buffer, to save pixels in
    let mut raw_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::new(size.x as u32, size.y as u32);

    let mut offset: u32 = 0;
    for tex in &mut shared.armature.textures {
        // get current texture as a buffer
        let img_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(
            tex.size.x as u32,
            tex.size.y as u32,
            tex.pixels.clone(),
        )
        .unwrap();

        // add it to the final buffer
        for x in 0..img_buf.width() {
            for y in 0..img_buf.height() {
                raw_buf.put_pixel(x + offset, y, *img_buf.get_pixel(x, y));
            }
        }

        tex.offset.x = offset as f32;

        // make sure the next texture will be added beside this one, instead of overwriting
        offset += img_buf.width();
    }

    // encode buffer to png, to allow saving it as a png file
    let mut png_buf: Vec<u8> = vec![];
    let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
    encoder
        .write_image(
            &raw_buf,
            raw_buf.width(),
            raw_buf.height(),
            image::ColorType::Rgba8.into(),
        )
        .unwrap();

    png_buf
}

pub fn prepare_files(shared: &mut Shared) -> (Vec2, String, Vec<u8>) {
    // get the image size in advance
    let mut size = Vec2::default();
    for tex in &shared.armature.textures {
        size.x += tex.size.x;
        if tex.size.y > size.y {
            size.y = tex.size.y;
        }
    }

    let mut png_buf = vec![];

    if size != Vec2::ZERO {
        png_buf = create_tex_sheet(shared, &size);
    }

    // clone armature and make some edits, then serialize it
    let mut armature_copy = shared.armature.clone();

    for bone in &mut armature_copy.bones {
        if bone.tex_idx == -1 {
            continue;
        }

        // if it is a regular rect, empty verts and indices
        if !bone_meshes_edited(
            armature_copy.textures[bone.tex_idx as usize].size,
            &bone.vertices,
        ) {
            bone.vertices = vec![];
            bone.indices = vec![];
        }
    }

    let root = Root {
        armatures: vec![armature_copy],
        texture_size: size,
    };

    let armatures_json = serde_json::to_string(&root).unwrap();

    (size, armatures_json, png_buf)
}

pub fn import<R: Read + std::io::Seek>(
    data: R,
    shared: &mut crate::Shared,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    bind_group_layout: &BindGroupLayout,
    context: &egui::Context,
) {
    let mut zip = zip::ZipArchive::new(data);
    if let Ok(_) = zip {
    } else {
        shared
            .ui
            .open_modal("That's not a SkelForm armature!".to_string(), false);
        return;
    }

    // load armature
    let armature_file = zip.as_mut().unwrap().by_name("armature.json").unwrap();
    let mut root: crate::Root = serde_json::from_reader(armature_file).unwrap();

    // load texture
    if root.armatures[0].textures.len() > 0 {
        let texture_file = zip.as_mut().unwrap().by_name("textures.png").unwrap();

        let mut bytes = vec![];
        for byte in texture_file.bytes() {
            bytes.push(byte.unwrap());
        }
        let mut img = image::load_from_memory(&bytes).unwrap();

        shared.bind_groups = vec![];
        shared.ui.texture_images = vec![];

        for texture in &mut root.armatures[0].textures {
            texture.pixels = img
                .crop(
                    texture.offset.x as u32,
                    0,
                    texture.size.x as u32,
                    texture.size.y as u32,
                )
                .into_rgba8()
                .to_vec();

            shared.bind_groups.push(renderer::create_texture_bind_group(
                texture.pixels.to_vec(),
                texture.size,
                queue,
                device,
                bind_group_layout,
            ));

            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [texture.size.x as usize, texture.size.y as usize],
                &texture.pixels,
            );
            let tex = context.load_texture("anim_icons", color_image, Default::default());
            shared.ui.texture_images.push(tex);
        }
    }

    shared.armature = root.armatures[0].clone();

    shared.unselect_everything();
    shared.set_tutorial_step(TutorialStep::None);
}

pub fn undo_redo(undo: bool, shared: &mut Shared) {
    let action: Action;
    if undo {
        if shared.undo_actions.last() == None {
            return;
        }
        action = shared.undo_actions.last().unwrap().clone();
    } else {
        if shared.redo_actions.last() == None {
            return;
        }
        action = shared.redo_actions.last().unwrap().clone();
    }
    let mut new_action = action.clone();

    match &action.action {
        ActionEnum::Bone => {
            if action.action_type == ActionType::Created {
                shared.selected_bone_idx = usize::MAX;
                if undo {
                    for (i, bone) in shared.armature.bones.iter().enumerate() {
                        if bone.id == action.id {
                            shared.armature.bones.remove(i);
                            break;
                        }
                    }
                } else {
                    armature_window::new_bone(shared, -1);
                }
            } else if (action.id as usize) <= shared.armature.bones.len() - 1 {
                new_action.bone = shared.armature.bones[action.id as usize].clone();
                *shared.find_bone_mut(action.id).unwrap() = action.bone.clone();

                for i in 0..shared.armature.bones.len() {
                    //shared.organize_bone(i);
                }
            }
        }
        ActionEnum::Animation => {
            if action.action_type == ActionType::Created {
                shared.ui.anim.selected = usize::MAX;
                if undo {
                    shared.armature.animations.pop();
                } else {
                    keyframe_editor::new_animation(shared);
                }
            } else if (action.id as usize) <= shared.armature.animations.len() - 1 {
                new_action.animation = shared.armature.animations[action.id as usize].clone();
                shared.armature.animations[action.id as usize] = action.animation.clone();
            }
        }
        _ => {}
    }

    if undo {
        shared.redo_actions.push(new_action);
        shared.undo_actions.pop();
    } else {
        shared.undo_actions.push(new_action);
        shared.redo_actions.pop();
    }
}

pub fn bone_meshes_edited(tex_size: Vec2, verts: &Vec<Vertex>) -> bool {
    let mut is_rect = true;
    for vert in verts {
        if vert.pos.x != 0. && vert.pos.x.abs() != tex_size.x.abs() {
            is_rect = false;
            break;
        }
        if vert.pos.y != 0. && vert.pos.y.abs() != tex_size.y.abs() {
            is_rect = false;
            break;
        }
    }
    !is_rect
}
