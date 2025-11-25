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

use image::{ExtendedColorType::Rgb8, GenericImage, ImageEncoder};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

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
pub fn open_save_dialog(file_name: &Arc<Mutex<String>>, saving: &Arc<Mutex<Saving>>) {
    let filename = Arc::clone(&file_name);
    let csaving = Arc::clone(&saving);
    std::thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("SkelForm Armature", &["skf"])
            .save_file();
        if task == None {
            return;
        }
        *filename.lock().unwrap() = task
            .as_ref()
            .unwrap()
            .as_path()
            .to_str()
            .unwrap()
            .to_string();
        *csaving.lock().unwrap() = shared::Saving::CustomPath;
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_import_dialog(file_name: &Arc<Mutex<String>>, file_contents: &Arc<Mutex<Vec<u8>>>) {
    let filename = Arc::clone(&file_name);
    let filecontents = Arc::clone(&file_contents);
    std::thread::spawn(move || {
        let task = rfd::FileDialog::new().pick_file();
        if task == None {
            return;
        }

        let file_str = task.as_ref().unwrap().as_path().to_str();
        *filename.lock().unwrap() = file_str.unwrap().to_string();
        *filecontents.lock().unwrap() =
            fs::read(task.unwrap().as_path().to_str().unwrap()).unwrap();
    });
}

#[cfg(target_arch = "wasm32")]
pub fn save_web(shared: &Shared) {
    let mut size = Vec2::default();
    let mut png_buf = vec![];
    let mut carmature = shared.armature.clone();

    if shared.armature.styles.len() > 0 && shared.armature.styles[0].textures.len() > 0 {
        (png_buf, size) = utils::create_tex_sheet(&mut carmature);
    }

    let (armatures_json, editor_json) = prepare_files(&carmature, shared.camera.clone(), size);

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

pub fn create_tex_sheet(armature: &mut Armature) -> (std::vec::Vec<u8>, Vec2) {
    let mut boxes = vec![];
    let mut size = 0;
    let mut placed = vec![];
    let mut tex_len = 0;

    for set in &armature.styles {
        for tex in &set.textures {
            boxes.push(max_rects::packing_box::PackingBox::new(
                tex.image.width() as i32,
                tex.image.height() as i32,
            ));
            tex_len += 1;
        }
    }

    while placed.len() != tex_len {
        size += 128;
        let bins = vec![max_rects::bucket::Bucket::new(size - 1, size - 1, 0, 0, 1)];
        let mut problem = max_rects::max_rects::MaxRects::new(boxes.clone(), bins.clone());
        (placed, _, _) = problem.place();
    }

    let mut raw_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::new(size as u32, size as u32);

    for set in &mut armature.styles {
        for tex in &mut set.textures {
            let p = placed
                .iter()
                .position(|pl| pl.width == tex.size.x as i32 && pl.height == tex.size.y as i32)
                .unwrap();

            let offset_x = placed[p].get_coords().0 as u32;
            let offset_y = placed[p].get_coords().2 as u32;

            // ensure another tex of the same size won't overwrite this one
            placed.remove(p);

            raw_buf.copy_from(&tex.image, offset_x, offset_y).unwrap();

            tex.offset = Vec2::new(offset_x as f32, offset_y as f32);
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

pub fn prepare_files(armature: &Armature, camera: Camera, tex_size: Vec2) -> (String, String) {
    // clone armature and make some edits, then serialize it
    let mut armature_copy = armature.clone();

    let mut family_ids: Vec<i32> = armature_copy
        .bones
        .iter()
        .map(|bone| bone.ik_family_id)
        .filter(|id| *id != -1)
        .collect();
    family_ids.dedup();

    let mut ik_root_ids = vec![];

    for fid in family_ids {
        let ac = &mut armature_copy;
        let joints: Vec<&Bone> = ac
            .bones
            .iter()
            .filter(|bone| bone.ik_family_id == fid)
            .collect();

        let mut bone_ids = vec![];
        for joint in &joints {
            let idx = ac.bones.iter().position(|bone| bone.id == joint.id);
            bone_ids.push(idx.unwrap() as i32);
        }

        let mut target_id = -1;
        let joint_target_id = joints[0].ik_target_id;
        let target_idx = ac.bones.iter().position(|bone| bone.id == joint_target_id);
        if target_idx != None {
            target_id = target_idx.unwrap() as i32;
        }

        let mut root_bone = ac.bones.iter_mut().find(|bone| bone.id == bone_ids[0]);
        root_bone.as_mut().unwrap().ik_bone_ids = bone_ids;
        root_bone.as_mut().unwrap().ik_target_id = target_id;
        root_bone.as_mut().unwrap().ik_constraint_id =
            root_bone.as_ref().unwrap().ik_constraint as i32;
        root_bone.as_mut().unwrap().ik_mode_id = root_bone.as_ref().unwrap().ik_mode as i32;
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
        // if it's a regular rect, empty verts and indices
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
            continue;
        }

        for w in 0..armature_copy.bones[b].binds.len() {
            let bone_id = armature_copy.bones[b].binds[w].bone_id;
            armature_copy.bones[b].binds[w].bone_id = armature_copy
                .bones
                .iter()
                .position(|bone| bone.id == bone_id)
                .unwrap() as i32;
            for v in 0..armature_copy.bones[b].binds[w].verts.len() {
                let vert_id = armature_copy.bones[b].binds[w].verts[v].id;
                armature_copy.bones[b].binds[w].verts[v].id = armature_copy.bones[b]
                    .vertices
                    .iter()
                    .position(|vert| vert.id == vert_id as u32)
                    .unwrap() as i32;
            }
        }

        for (i, vert) in armature_copy.bones[b].vertices.iter_mut().enumerate() {
            vert.init_pos = vert.pos;
            vert.id = i as u32;
        }
    }

    for bone in &mut armature_copy.bones {
        bone.init_pos = bone.pos;
        bone.init_rot = bone.rot;
        bone.init_scale = bone.scale;
        bone.init_constraint = bone.ik_constraint_id;
        bone.init_hidden = bone.hidden;

        if bone.ik_bone_ids.len() == 0 {
            bone.ik_constraint = JointConstraint::Skip;
            bone.ik_constraint_id = -1;
            bone.ik_mode = InverseKinematicsMode::Skip;
            bone.ik_mode_id = -1;
            bone.ik_family_id = -1;
            bone.init_constraint = -1;
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

    for bone in &armature_copy.bones {
        if bone.ik_family_id != -1 {
            ik_root_ids.push(bone.id);
        }
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
        texture_size: Vec2I::new(tex_size.x as i32, tex_size.y as i32),
        ik_root_ids,
        bones: armature_copy.bones,
        animations: armature_copy.animations,
        styles: armature_copy.styles,
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

    (armatures_json, editor_json)
}

pub fn import<R: Read + std::io::Seek>(
    data: R,
    shared: &mut crate::Shared,
    queue: Option<&wgpu::Queue>,
    device: Option<&wgpu::Device>,
    bind_group_layout: Option<&BindGroupLayout>,
    context: Option<&egui::Context>,
) {
    let mut zip = zip::ZipArchive::new(data);
    if let Err(_) = zip {
        return;
    }

    // load armature
    let armature_file = zip.as_mut().unwrap().by_name("armature.json").unwrap();
    let root: crate::Root = serde_json::from_reader(armature_file).unwrap();

    shared.armature = shared::Armature {
        bones: root.bones,
        animations: root.animations,
        styles: root.styles,
    };

    for bone in &mut shared.armature.bones {
        for (i, vert) in bone.vertices.iter_mut().enumerate() {
            vert.id = i as u32;
        }
    }

    // populate style ids
    for s in 0..shared.armature.styles.len() {
        shared.armature.styles[s].id = s as i32;
    }

    // populate bone IK data
    for b in 0..shared.armature.bones.len() {
        if shared.armature.bones[b].ik_bone_ids.len() > 0 {
            for i in 0..shared.armature.bones[b].ik_bone_ids.len() {
                let id = shared.armature.bones[b].ik_bone_ids[i];
                shared
                    .armature
                    .bones
                    .iter_mut()
                    .find(|bone| bone.id == id as i32)
                    .unwrap()
                    .ik_family_id = shared.armature.bones[b].ik_family_id;
            }
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
    let styles = &shared.armature.styles;
    let has_tex = styles.iter().find(|set| set.textures.len() > 0) != None;
    if styles.len() > 0 && has_tex {
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

                if queue != None && device != None && bind_group_layout != None {
                    tex.bind_group = Some(renderer::create_texture_bind_group(
                        tex.image.clone().into_rgba8().to_vec(),
                        tex.size,
                        queue.unwrap(),
                        device.unwrap(),
                        bind_group_layout.unwrap(),
                    ));
                }

                if context == None {
                    continue;
                }

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
                let ui_tex =
                    context
                        .unwrap()
                        .load_texture("anim_icons", color_image, Default::default());
                tex.ui_img = Some(ui_tex);
            }
        }
    }

    shared.ui.unselect_everything();
    shared.ui.set_state(UiState::StartupWindow, false);
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
            let bones = &mut shared.armature.bones;
            if bones.len() == 0 || shared.ui.selected_bone_idx > bones.len() - 1 {
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
            let animations = &mut shared.armature.animations;
            if animations.len() == 0 || shared.ui.anim.selected > animations.len() - 1 {
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
        if vert.pos.x.abs() != tex_size.x / 2. {
            is_rect = false;
            break;
        }
        if vert.pos.y.abs() != tex_size.y / 2. {
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
        let url = bin_path() + docs_name + "/" + _path;
        println!("{}", url);
        match open::that(url) {
            Err(_) => {
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
        updateUiSlider();
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

pub fn after_underscore(str: &str) -> &str {
    str.split('_').collect::<Vec<_>>()[1]
}

pub fn without_unicode(str: &str) -> &str {
    str.split('\u{0000}').collect::<Vec<_>>()[0]
}

pub fn process_thumbnail(
    buffer: &wgpu::Buffer,
    device: &wgpu::Device,
    resolution: Vec2,
) -> Vec<u8> {
    // wait for screenshot buffer to complete
    let _ = device.poll(wgpu::PollType::Wait);

    let view = buffer.slice(..).get_mapped_range();

    let mut rgb = vec![0u8; (resolution.x * resolution.y * 3.) as usize];
    for (j, chunk) in view.as_ref().chunks_exact(4).enumerate() {
        let offset = j * 3;
        if offset + 2 > rgb.len() {
            return vec![];
        }
        rgb[offset + 0] = chunk[2];
        rgb[offset + 1] = chunk[1];
        rgb[offset + 2] = chunk[0];
    }

    type ImgType = image::ImageBuffer<image::Rgb<u8>, Vec<u8>>;
    let img_buf =
        <ImgType>::from_raw(resolution.x as u32, resolution.y as u32, rgb.clone()).unwrap();

    let thumb_size = Vec2::new(128., 128.);
    let mut img = image::DynamicImage::ImageRgb8(img_buf);
    img = img.thumbnail(thumb_size.x as u32, thumb_size.y as u32);

    let mut thumb_buf: Vec<u8> = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut thumb_buf);
    encoder
        .write_image(img.to_rgb8().as_raw(), img.width(), img.height(), Rgb8)
        .unwrap();

    thumb_buf
}

pub fn markdown(str: String, local_doc_url: String) -> String {
    let user_docs = if local_doc_url != "" {
        local_doc_url.clone() + "user-docs"
    } else {
        "https://skelform.org/user-docs".to_string()
    };
    str.replace("user-docs", &user_docs)
}

pub fn get_all_parents(bones: &Vec<Bone>, bone_id: i32) -> Vec<Bone> {
    // add own bone temporarily
    let bone = bones.iter().find(|b| b.id == bone_id).unwrap().clone();
    let mut parents: Vec<Bone> = vec![bone];

    while parents.last().unwrap().parent_id != -1 {
        let pid = parents.last().unwrap().parent_id;
        parents.push(bones.iter().find(|bone| bone.id == pid).unwrap().clone());
    }

    // remove own bone from list
    parents.remove(0);

    parents
}
