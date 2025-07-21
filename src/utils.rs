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

use std::{
    io::{Read, Write},
    ops::Index,
};

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
pub fn open_save_dialog(temp_save_path: String) {
    std::thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("SkelForm Armature", &["skf"])
            .save_file();
        if task == None {
            return;
        }
        file_reader::create_temp_file(&temp_save_path, task.unwrap().as_path().to_str().unwrap());
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_import_dialog(temp_file_to_write: String) {
    std::thread::spawn(move || {
        let task = rfd::FileDialog::new().pick_file();
        if task == None {
            return;
        }
        file_reader::create_temp_file(
            &temp_file_to_write,
            task.unwrap().as_path().to_str().unwrap(),
        );
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(path: String, armature: &Armature) {
    let (size, armatures_json, png_buf) = prepare_files(armature);

    // create zip file
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());

    let options =
        zip::write::FullFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // save armature json and texture image
    zip.start_file("armature.json", options.clone()).unwrap();
    zip.write(armatures_json.as_bytes()).unwrap();
    if size != Vec2::ZERO {
        zip.start_file("textures.png", options).unwrap();
        zip.write(&png_buf).unwrap();
    }

    zip.finish().unwrap();
}

#[cfg(target_arch = "wasm32")]
pub fn save_web(armature: &Armature) {
    let (size, armatures_json, png_buf) = prepare_files(armature);

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

fn create_tex_sheet(armature: &mut Armature, size: &Vec2) -> std::vec::Vec<u8> {
    // set up the buffer, to save pixels in
    let mut raw_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::new(size.x as u32, size.y as u32);

    let mut offset: u32 = 0;
    for tex in &mut armature.textures {
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

pub fn prepare_files(armature: &Armature) -> (Vec2, String, Vec<u8>) {
    // get the image size in advance
    let mut size = Vec2::default();
    for tex in &armature.textures {
        size.x += tex.size.x;
        if tex.size.y > size.y {
            size.y = tex.size.y;
        }
    }

    let mut png_buf = vec![];

    // clone armature and make some edits, then serialize it
    let mut armature_copy = armature.clone();

    if size != Vec2::ZERO {
        png_buf = create_tex_sheet(&mut armature_copy, &size);
    }

    for bone in &mut armature_copy.bones {
        // if it is a regular rect, empty verts and indices
        if bone.tex_idx == -1
            || !bone_meshes_edited(
                armature_copy.textures[bone.tex_idx as usize].size,
                &bone.vertices,
            )
        {
            bone.vertices = vec![];
            bone.indices = vec![];
        }
    }

    let root = Root {
        armature: armature_copy,
        texture_size: size,
    };

    let armatures_json = serde_json::to_string(&root).unwrap();

    (size, armatures_json, png_buf)
}

pub fn import<R: Read + std::io::Seek>(
    path: &str,
    data: R,
    shared: &mut crate::Shared,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    bind_group_layout: &BindGroupLayout,
    context: &egui::Context,
) {
    let mut zip = zip::ZipArchive::new(data);
    let mut ok = false;
    if let Ok(_) = zip {
        ok = true;
    }
    let ext = path.split('.').last().unwrap();
    if ext != "skf" {
        ok = false;
        if ext == "psd" {
            #[cfg(not(target_arch = "wasm32"))]
            file_reader::create_temp_file(&shared.temp_path.import_psd, path);

            return;
        }
    }

    if !ok {
        let text = "File could not be parsed.\n\nSupported files:\n- SkelForm armature (.skf)\n- Photoshop Document (.psd)";
        shared.ui.open_modal(text.to_string(), false);
        file_reader::del_temp_files(&shared.temp_path.base);
        return;
    }

    // load armature
    let armature_file = zip.as_mut().unwrap().by_name("armature.json").unwrap();
    let root: crate::Root = serde_json::from_reader(armature_file).unwrap();

    shared.armature = root.armature.clone();
    for b in 0..shared.armature.bones.len() {
        let mut children = vec![];
        armature_window::get_all_children(
            &shared.armature.bones,
            &mut children,
            &shared.armature.bones[b],
        );
        shared.armature.bones[b].folded = children.len() > 0;
    }

    // load texture
    if root.armature.textures.len() > 0 {
        let texture_file = zip.as_mut().unwrap().by_name("textures.png").unwrap();

        let mut bytes = vec![];
        for byte in texture_file.bytes() {
            bytes.push(byte.unwrap());
        }
        let mut img = image::load_from_memory(&bytes).unwrap();

        shared.armature.bind_groups = vec![];
        shared.ui.texture_images = vec![];

        for texture in &mut shared.armature.textures {
            texture.pixels = img
                .crop(
                    texture.offset.x as u32,
                    0,
                    texture.size.x as u32,
                    texture.size.y as u32,
                )
                .into_rgba8()
                .to_vec();

            shared
                .armature
                .bind_groups
                .push(renderer::create_texture_bind_group(
                    texture.pixels.to_vec(),
                    texture.size,
                    queue,
                    device,
                    bind_group_layout,
                ));

            let pixels = img
                .crop(
                    texture.offset.x as u32,
                    0,
                    texture.size.x as u32,
                    texture.size.y as u32,
                )
                .resize_exact(300, 300, image::imageops::FilterType::Nearest)
                .into_rgba8()
                .to_vec();

            let color_image = egui::ColorImage::from_rgba_unmultiplied([300, 300], &pixels);
            let tex = context.load_texture("anim_icons", color_image, Default::default());
            shared.ui.texture_images.push(tex);
        }
    }

    shared.ui.unselect_everything();
    shared.ui.set_tutorial_step(TutorialStep::None);

    file_reader::del_temp_files(&shared.temp_path.base);
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
            new_action.bones = vec![shared.armature.bones[action.id as usize].clone()];
            *shared.armature.find_bone_mut(action.id).unwrap() = action.bones[0].clone();

            for _ in 0..shared.armature.bones.len() {
                //shared.organize_bone(i);
            }
        }
        ActionEnum::Bones => {
            new_action.bones = shared.armature.bones.clone();
            shared.armature.bones = action.bones.clone();
            if shared.armature.bones.len() == 0
                || shared.ui.selected_bone_idx > shared.armature.bones.len() - 1
            {
                shared.ui.selected_bone_idx = usize::MAX;
            }
        }
        ActionEnum::Animation => {
            new_action.animations = vec![shared.armature.animations[action.id as usize].clone()];
            *shared.armature.find_anim_mut(action.id).unwrap() = action.animations[0].clone();
        }
        ActionEnum::Animations => {
            new_action.animations = shared.armature.animations.clone();
            shared.armature.animations = action.animations.clone();
            if shared.armature.animations.len() == 0
                || shared.ui.anim.selected > shared.armature.animations.len() - 1
            {
                shared.ui.anim.selected = usize::MAX;
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

pub fn open_docs(is_dev: bool, path: &str) {
    let docs_name = if is_dev { "user_docs" } else { "dev_docs" };
    #[cfg(target_arch = "wasm32")]
    openDocumentation(docs_name.to_string());
    // open the local docs, or online if it can't be found on default path
    #[cfg(not(target_arch = "wasm32"))]
    {
        match open::that(bin_path() + docs_name + "/index.html" + &path.to_string()) {
            Err(_) => match open::that(
                "https://retropaint.github.io/skelform_".to_string()
                    + docs_name
                    + "/"
                    + &path.to_string(),
            ) {
                Err(_) => println!("couldn't open"),
                Ok(file) => file,
            },
            Ok(file) => file,
        };
    }
}

pub fn bin_path() -> String {
    let mut bin = std::env::current_exe()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // remove executable from path
    let _ = bin.split_off(bin.find("SkelForm").unwrap());

    if cfg!(target_os = "macos") {
        bin.push_str("SkelForm.app/Contents/MacOS/")
    }

    bin
}
