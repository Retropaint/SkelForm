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

use image::{GenericImage, ImageEncoder};

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
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(left, top),
            uv: Vec2::new(0., 1.),
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(left, bot),
            uv: Vec2::new(0., 0.),
            ..Default::default()
        },
        Vertex {
            pos: Vec2::new(right, bot),
            uv: Vec2::new(1., 1.),
            ..Default::default()
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

#[cfg(target_arch = "wasm32")]
pub fn save_web(shared: &Shared) {
    let (size, armatures_json, editor_json, png_buf) =
        prepare_files(&shared.armature, shared.camera.clone());

    // create zip file
    let mut buf: Vec<u8> = Vec::new();
    let cursor = std::io::Cursor::new(&mut buf);
    let mut zip = zip::ZipWriter::new(cursor);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // save armature json and texture image
    zip.start_file("armature.json", options).unwrap();
    zip.write(armatures_json.as_bytes()).unwrap();
    zip.start_file("editor.json", options).unwrap();
    zip.write(editor_json.as_bytes()).unwrap();
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

    for set in &armature.styles {
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

    // todo:
    // Texture atlas is the biggest bottleneck in saving time. Both could be improved:
    // - coping individual textures to the final image
    // - encoding to png (mandatory for regular saving, but autosaving could use another format like bmp)

    for set in &mut armature.styles {
        for tex in &mut set.textures {
            let offset_x = packed.as_ref().unwrap().packed_locations()[&idx].1.x();
            let offset_y = packed.as_ref().unwrap().packed_locations()[&idx].1.y();

            raw_buf.copy_from(&tex.image, offset_x, offset_y).unwrap();

            tex.offset = Vec2::new(offset_x as f32, offset_y as f32);

            idx += 1;
        }
    }

    // encode buffer to png
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

pub fn prepare_files(armature: &Armature, camera: Camera) -> (Vec2, String, String, Vec<u8>) {
    let mut png_buf = vec![];
    let mut size = Vec2::new(0., 0.);

    // clone armature and make some edits, then serialize it
    let mut armature_copy = armature.clone();

    armature_copy.ik_families = vec![];

    let mut family_ids: Vec<i32> = armature_copy
        .bones
        .iter()
        .map(|bone| bone.ik_family_id)
        .filter(|id| *id != -1)
        .collect();
    family_ids.dedup();

    for fid in family_ids {
        let joints: Vec<&Bone> = armature_copy
            .bones
            .iter()
            .filter(|bone| bone.ik_family_id == fid)
            .collect();

        let mut bone_ids = vec![];
        for joint in &joints {
            let idx = armature_copy
                .bones
                .iter()
                .position(|bone| bone.id == joint.id)
                .unwrap();
            bone_ids.push(idx as i32);
        }

        let target_id = joints[0].ik_target_id;
        let constraint = joints[0].constraint;

        armature_copy.ik_families.push(IkFamily {
            constraint,
            target_id,
            bone_ids,
        })
    }

    // populate keyframe bone_idx
    for a in 0..armature_copy.animations.len() {
        for kf in 0..armature_copy.animations[a].keyframes.len() {
            let keyframe = &mut armature_copy.animations[a].keyframes[kf];
            let bones = &mut armature_copy.bones.iter();
            keyframe.bone_id = bones.position(|bone| bone.id == keyframe.bone_id).unwrap() as i32;
        }
    }

    for b in 0..armature_copy.bones.len() {
        // if it is a regular rect, empty verts and indices
        if armature_copy.get_current_tex(armature_copy.bones[b].id) == None
            || !bone_meshes_edited(
                armature_copy
                    .get_current_tex(armature_copy.bones[b].id)
                    .unwrap()
                    .size,
                &armature_copy.bones[b].vertices,
            )
        {
            armature_copy.bones[b].vertices = vec![];
            armature_copy.bones[b].indices = vec![];
        }
    }

    for b in 0..armature_copy.bones.len() {
        if armature_copy.bones[b].parent_id == -1 {
            continue;
        }

        armature_copy.bones[b].parent_id = armature_copy
            .bones
            .iter()
            .position(|bone| bone.id == armature_copy.bones[b].parent_id)
            .unwrap() as i32;
    }

    // restructure bone ids
    for b in 0..armature_copy.bones.len() {
        let bone = &mut armature_copy.bones[b];
        if bone.style_ids.len() == 0 {
            bone.tex_idx = -1;
            bone.zindex = -1;
        }

        bone.id = b as i32;
    }

    for bone in &mut armature_copy.bones {
        bone.init_pos = bone.pos;
        bone.init_rot = bone.rot;
        bone.init_scale = bone.scale;
    }

    if armature.styles.len() > 0 && armature.styles[0].textures.len() > 0 {
        (png_buf, size) = create_tex_sheet(&mut armature_copy);
    }

    // populate texture ser_offset and ser_size
    for s in 0..armature.styles.len() {
        for t in 0..armature.styles[s].textures.len() {
            let tex = &mut armature_copy.styles[s].textures[t];
            tex.ser_offset = Vec2I::new(tex.offset.x as i32, tex.offset.y as i32);
            tex.ser_size = Vec2I::new(tex.size.x as i32, tex.size.y as i32);
        }
    }

    let root = Root {
        version: env!("CARGO_PKG_VERSION").to_string(),
        armature: armature_copy,
        texture_size: Vec2I::new(size.x as i32, size.y as i32),
    };

    let armatures_json = serde_json::to_string(&root).unwrap();

    // iterable editor bone exports
    let mut editor = EditorOptions {
        camera,
        bones: vec![],
        styles: vec![],
    };
    for bone in &armature.bones {
        editor.bones.push(EditorBone {
            folded: bone.folded,
            ik_folded: bone.ik_folded,
            meshdef_folded: bone.meshdef_folded,
            ik_disabled: bone.ik_disabled,
        });
    }
    for style in &armature.styles {
        editor.styles.push(EditorStyle {
            active: style.active,
        })
    }
    let editor_json = serde_json::to_string(&editor).unwrap();

    (size, armatures_json, editor_json, png_buf)
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
    if let Err(_) = zip {
        return;
    }

    // load armature
    let armature_file = zip.as_mut().unwrap().by_name("armature.json").unwrap();
    let root: crate::Root = serde_json::from_reader(armature_file).unwrap();

    shared.armature = root.armature.clone();

    // populate style ids
    for s in 0..shared.armature.styles.len() {
        shared.armature.styles[s].id = s as i32;
    }

    // populate bone IK data from ik_families
    for f in 0..shared.armature.ik_families.len() {
        let family = &shared.armature.ik_families[f];
        let target_id = if let Some(target) = shared.armature.find_bone(family.target_id) {
            target.id
        } else {
            -1
        };

        for i in 0..family.bone_ids.len() {
            let bone = &mut shared.armature.bones[family.bone_ids[i] as usize];
            bone.ik_family_id = f as i32;
            bone.constraint = family.constraint;
            bone.ik_target_id = target_id;
        }
    }

    // load editor data
    if let Ok(editor_file) = zip.as_mut().unwrap().by_name("editor.json") {
        let editor: crate::EditorOptions = serde_json::from_reader(editor_file).unwrap();

        shared.camera = editor.camera;

        for b in 0..shared.armature.bones.len() {
            let bone = &mut shared.armature.bones[b];
            let ed_bone = &editor.bones[b];

            // iterable editor bone imports
            bone.folded = ed_bone.folded;
            bone.ik_folded = ed_bone.ik_folded;
            bone.meshdef_folded = ed_bone.meshdef_folded;
            bone.ik_disabled = ed_bone.ik_disabled;
        }

        for s in 0..shared.armature.styles.len() {
            let style = &mut shared.armature.styles[s];
            let ed_style = &editor.styles[s];

            style.active = ed_style.active;
        }
    }

    // load texture
    let has_tex = root
        .armature
        .styles
        .iter()
        .find(|set| set.textures.len() > 0)
        != None;
    if root.armature.styles.len() > 0 && has_tex {
        let texture_file = zip.as_mut().unwrap().by_name("textures.png").unwrap();

        let mut bytes = vec![];
        for byte in texture_file.bytes() {
            bytes.push(byte.unwrap());
        }
        let mut img = image::load_from_memory(&bytes).unwrap();

        for set in &mut shared.armature.styles {
            for tex in &mut set.textures {
                tex.offset = Vec2::new(tex.ser_offset.x as f32, tex.ser_offset.y as f32);
                tex.size = Vec2::new(tex.ser_size.x as f32, tex.ser_size.y as f32);

                tex.image = img.crop(
                    tex.offset.x as u32,
                    tex.offset.y as u32,
                    tex.size.x as u32,
                    tex.size.y as u32,
                );

                tex.bind_group = Some(renderer::create_texture_bind_group(
                    tex.image.clone().into_rgba8().to_vec(),
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
    shared.ui.set_state(UiState::StartupWindow, false);

    file_reader::del_temp_files(&shared.temp_path.base);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_to_recent_files(paths: &Vec<String>) {
    fs::create_dir_all(recents_path().parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(&recents_path()).unwrap();
    file.write_all(serde_json::to_string(&paths).unwrap().as_bytes())
        .unwrap();
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
        ActionType::Bone => {
            new_action.bones = vec![shared.armature.bones[action.id as usize].clone()];
            *shared.armature.find_bone_mut(action.id).unwrap() = action.bones[0].clone();
        }
        ActionType::Bones => {
            new_action.bones = shared.armature.bones.clone();
            shared.armature.bones = action.bones.clone();
            if shared.armature.bones.len() == 0
                || shared.ui.selected_bone_idx > shared.armature.bones.len() - 1
            {
                shared.ui.selected_bone_idx = usize::MAX;
            }
        }
        ActionType::Animation => {
            new_action.animations = vec![shared.armature.animations[action.id as usize].clone()];
            *shared.armature.find_anim_mut(action.id).unwrap() = action.animations[0].clone();
        }
        ActionType::Animations => {
            new_action.animations = shared.armature.animations.clone();
            shared.armature.animations = action.animations.clone();
            if shared.armature.animations.len() == 0
                || shared.ui.anim.selected > shared.armature.animations.len() - 1
            {
                shared.ui.anim.selected = usize::MAX;
            }
        }
        ActionType::TextureSet => {
            new_action.tex_sets = vec![shared.armature.styles[action.id as usize].clone()];
            shared.armature.styles[action.id as usize] = action.tex_sets[0].clone();
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

pub fn open_docs(is_dev: bool, mut _path: &str) {
    let docs_name = if is_dev { "dev-docs" } else { "user-docs" };
    #[cfg(target_arch = "wasm32")]
    openDocumentation(docs_name.to_string(), _path.to_string());
    // open the local docs, or online if it can't be found on default path
    #[cfg(not(target_arch = "wasm32"))]
    {
        if _path == "" {
            _path = "index.html";
        }
        let url = bin_path() + docs_name + "/" + _path;
        match open::that(url) {
            Err(_) => {
                if _path == "index.html" {
                    _path = "";
                }
                let url =
                    "https://skelform.org/".to_string() + docs_name + "/" + &_path.to_string();
                match open::that(url) {
                    Err(_) => println!("couldn't open"),
                    Ok(file) => file,
                }
            }
            Ok(file) => file,
        };
    }
}

pub fn bin_path() -> String {
    #[cfg(feature = "debug")]
    return "".to_string();

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
    img_buf: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
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

/// Recursively flattens a JSON object into dotted keys
pub fn flatten_json(
    value: &serde_json::Value,
    prefix: String,
    out: &mut std::collections::HashMap<String, String>,
    suffix: String,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let new_prefix = if prefix.is_empty() {
                    k.to_string()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json(v, new_prefix, out, suffix.clone());
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix, s.clone() + &suffix);
        }
        _ => {
            // only strings should be in your loc json
            // but you can handle numbers/bools here if you want
        }
    }
}
