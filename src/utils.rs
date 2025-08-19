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
            color: VertexColor::default(),
        },
        Vertex {
            pos: Vec2::new(left, top),
            uv: Vec2::new(0., 1.),
            color: VertexColor::default(),
        },
        Vertex {
            pos: Vec2::new(left, bot),
            uv: Vec2::new(0., 0.),
            color: VertexColor::default(),
        },
        Vertex {
            pos: Vec2::new(right, bot),
            uv: Vec2::new(1., 1.),
            color: VertexColor::default(),
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, Ord, PartialOrd)]
enum RectGroupId {
    GroupIdOne,
}

fn create_tex_sheet(armature: &mut Armature) -> (std::vec::Vec<u8>, Vec2) {
    // add textures to sheet generator
    let mut img_rect: rectangle_pack::GroupedRectsToPlace<i32, RectGroupId> =
        rectangle_pack::GroupedRectsToPlace::new();
    let mut idx = 0;

    for set in &armature.texture_sets {
        for tex in &set.textures {
            img_rect.push_rect(
                idx,
                None,
                rectangle_pack::RectToInsert::new(tex.size.x as u32, tex.size.y as u32, 1),
            );
            idx += 1;
        }
    }

    // keep generating sheet until the size is big enough
    let mut size = 32;
    let mut packed: Option<rectangle_pack::RectanglePackOk<i32, RectGroupId>> = None;
    while packed == None {
        let mut target_bins = std::collections::BTreeMap::new();
        target_bins.insert(
            RectGroupId::GroupIdOne,
            rectangle_pack::TargetBin::new(size, size, 1),
        );
        match rectangle_pack::pack_rects(
            &img_rect,
            &mut target_bins,
            &rectangle_pack::volume_heuristic,
            &mut rectangle_pack::contains_smallest_box,
        ) {
            Ok(data) => {
                packed = Some(data);
            }
            Err(_) => {
                println!("Tried texture atlas ({}, {})", size, size);
                size *= 2
            }
        }
    }

    let mut raw_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::new(size, size);

    let mut idx = 0;

    for set in &mut armature.texture_sets {
        for tex in &mut set.textures {
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
                    raw_buf.put_pixel(
                        x + packed.as_ref().unwrap().packed_locations()[&idx].1.x(),
                        y + packed.as_ref().unwrap().packed_locations()[&idx].1.y(),
                        *img_buf.get_pixel(x, y),
                    );
                }
            }

            tex.offset.x = packed.as_ref().unwrap().packed_locations()[&idx].1.x() as f32;
            tex.offset.y = packed.as_ref().unwrap().packed_locations()[&idx].1.y() as f32;

            idx += 1;
        }
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

    (png_buf, Vec2::new(size as f32, size as f32))
}

pub fn prepare_files(armature: &Armature) -> (Vec2, String, Vec<u8>) {
    let mut png_buf = vec![];
    let mut size = Vec2::new(0., 0.);

    // clone armature and make some edits, then serialize it
    let mut armature_copy = armature.clone();

    if armature.texture_sets.len() > 0 && armature.texture_sets[0].textures.len() > 0 {
        (png_buf, size) = create_tex_sheet(&mut armature_copy);
    }

    for bone in &mut armature_copy.bones {
        // if it is a regular rect, empty verts and indices
        if bone.tex_set_idx == -1
            || !bone_meshes_edited(
                armature_copy.texture_sets[bone.tex_set_idx as usize].textures
                    [bone.tex_idx as usize]
                    .size,
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
    if root.armature.texture_sets.len() > 0 {
        let texture_file = zip.as_mut().unwrap().by_name("textures.png").unwrap();

        let mut bytes = vec![];
        for byte in texture_file.bytes() {
            bytes.push(byte.unwrap());
        }
        let mut img = image::load_from_memory(&bytes).unwrap();

        for set in &mut shared.armature.texture_sets {
            for tex in &mut set.textures {
                tex.pixels = img
                    .crop(
                        tex.offset.x as u32,
                        tex.offset.y as u32,
                        tex.size.x as u32,
                        tex.size.y as u32,
                    )
                    .into_rgba8()
                    .to_vec();

                tex.bind_group = Some(renderer::create_texture_bind_group(
                    tex.pixels.to_vec(),
                    tex.size,
                    queue,
                    device,
                    bind_group_layout,
                ));

                let pixels = img
                    .crop(
                        tex.offset.x as u32,
                        tex.offset.y as u32,
                        tex.size.x as u32,
                        tex.size.y as u32,
                    )
                    .resize_exact(300, 300, image::imageops::FilterType::Nearest)
                    .into_rgba8()
                    .to_vec();

                let color_image = egui::ColorImage::from_rgba_unmultiplied([300, 300], &pixels);
                let ui_tex = context.load_texture("anim_icons", color_image, Default::default());
                tex.ui_img = Some(ui_tex);
            }
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

pub fn open_docs(is_dev: bool, _path: &str) {
    let docs_name = if is_dev { "user_docs" } else { "dev_docs" };
    #[cfg(target_arch = "wasm32")]
    openDocumentation(docs_name.to_string());
    // open the local docs, or online if it can't be found on default path
    #[cfg(not(target_arch = "wasm32"))]
    {
        match open::that(bin_path() + docs_name + "/index.html" + &_path.to_string()) {
            Err(_) => match open::that(
                "https://retropaint.github.io/skelform_".to_string()
                    + docs_name
                    + "/"
                    + &_path.to_string(),
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

pub fn save_config(config: &Config) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        fs::create_dir_all(config_path().parent().unwrap()).unwrap();
        let mut file = std::fs::File::create(&config_path()).unwrap();
        file.write_all(serde_json::to_string(&config).unwrap().as_bytes())
            .unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    {
        saveConfig(serde_json::to_string(config).unwrap());
    }
}

pub fn import_config(shared: &mut Shared) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut str = String::new();
        std::fs::File::open(&config_path())
            .unwrap()
            .read_to_string(&mut str)
            .unwrap();
        shared.config = serde_json::from_str(&str).unwrap_or_default();
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(data) = serde_json::from_str(&getConfig()) {
            shared.config = data;
        }
    }
}

pub fn add_texture_img(
    ctx: &egui::Context,
    img_buf: image::ImageBuffer<Rgba<u8>, Vec<u8>>,
    size: Vec2,
) -> egui::TextureHandle {
    // force 300x300 to texture size
    let resized = image::imageops::resize(
        &img_buf,
        size.x as u32,
        size.y as u32,
        image::imageops::FilterType::Nearest,
    );
    let color_image = egui::ColorImage::from_rgba_unmultiplied([300, 300], &resized);
    let tex = ctx.load_texture("anim_icons", color_image, Default::default());
    tex
}
