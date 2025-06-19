//! Reading uploaded images to turn into textures.
// test

use std::any::Any;

use image::buffer::ConvertBuffer;
use wgpu::*;

use crate::*;

// web-only imports
#[cfg(target_arch = "wasm32")]
mod web {
    pub use wasm_bindgen::prelude::wasm_bindgen;
    pub use web_sys::*;
}

#[cfg(target_arch = "wasm32")]
pub use web::*;

macro_rules! temp_file {
    ($name:ident, $path:expr) => {
        pub const $name: &str = $path;
    };
}

#[rustfmt::skip] temp_file!(TEMP_IMG_PATH,         ".skelform_img_path");
#[rustfmt::skip] temp_file!(TEMP_SAVE_PATH,        ".skelform_save_path");
#[rustfmt::skip] temp_file!(TEMP_EXPORT_VID_TEXT,  ".skelform_export_vid_text");
#[rustfmt::skip] temp_file!(TEMP_IMPORT_PATH,      ".skelform_import_path");
#[rustfmt::skip] temp_file!(TEMP_IMPORT_PSD_PATH, ".skelform_import_tiff_path");

pub const FILES: [&str; 5] = [
    TEMP_IMG_PATH,
    TEMP_SAVE_PATH,
    TEMP_IMPORT_PATH,
    TEMP_IMPORT_PSD_PATH,
    TEMP_EXPORT_VID_TEXT,
];

pub const EXPORT_VID_DONE: &str = "Done!";
pub const IMPORT_IMG_ERR: &str = "Could not extract image data.";

pub fn read(shared: &mut Shared, renderer: &Option<Renderer>, context: &egui::Context) {
    macro_rules! func {
        ($func:expr) => {
            $func(
                shared,
                &renderer.as_ref().unwrap().gpu.queue,
                &renderer.as_ref().unwrap().gpu.device,
                &renderer.as_ref().unwrap().bind_group_layout,
                context,
            )
        };
    }

    if let Some(_) = renderer.as_ref() {
        func!(read_image_loaders);
        func!(read_psd);

        #[cfg(target_arch = "wasm32")]
        func!(load_file);

        #[cfg(not(target_arch = "wasm32"))]
        {
            read_save(shared);
            func!(read_import);
            read_exported_video_frame(shared);
        }
    }
}
/// read temporary files created from file dialogs (native & WASM)
pub fn read_image_loaders(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context,
) {
    #[allow(unused_assignments)]
    let mut pixels: Vec<u8> = vec![];
    #[allow(unused_assignments)]
    let mut dimensions = Vec2::default();
    #[allow(unused_assignments, unused_mut)]
    let mut name = "".to_string();

    #[cfg(not(target_arch = "wasm32"))]
    {
        if !fs::exists(TEMP_IMG_PATH).unwrap() {
            return;
        }

        // delete files if selected bone is invalid
        if shared.armature.bones.len() == 0
            || shared.ui.selected_bone_idx > shared.armature.bones.len() - 1
        {
            del_temp_files();
            return;
        }

        let img_path = fs::read_to_string(TEMP_IMG_PATH).unwrap();
        if img_path == "" {
            del_temp_files();
            return;
        }

        // extract name
        let filename = img_path.split('/').last().unwrap().to_string();
        name = filename.split('.').collect::<Vec<_>>()[0].to_string();

        // read image pixels and dimensions
        let file_bytes = fs::read(img_path);
        let diffuse_image = image::load_from_memory(&file_bytes.unwrap()).unwrap();
        let rgba = diffuse_image.to_rgba8();
        pixels = rgba.as_bytes().to_vec();
        dimensions = Vec2::new(diffuse_image.width() as f32, diffuse_image.height() as f32);

        del_temp_files();
    }

    #[cfg(target_arch = "wasm32")]
    {
        if let Some((wasm_pixels, dims)) = load_image_wasm("last-image".to_string()) {
            dimensions = Vec2::new(dims.x as f32, dims.y as f32);
            pixels = wasm_pixels;
        } else {
            return;
        }

        name = getImgName().split('.').collect::<Vec<_>>()[0].to_string();

        removeImage();
    }

    if pixels.len() == 0 {
        shared.ui.open_modal(IMPORT_IMG_ERR.to_string(), false);
        return;
    }

    shared.ui.set_state(UiState::ImageModal, false);

    // check if this texture already exists
    for tex in &shared.armature.textures {
        if pixels == tex.pixels {
            return;
        }
    }

    add_texture(
        pixels,
        dimensions,
        &name,
        &mut shared.ui,
        &mut shared.armature,
        queue,
        device,
        bind_group_layout,
        ctx,
    );

    let tex_idx = shared.armature.textures.len() - 1;
    shared
        .armature
        .set_bone_tex(shared.selected_bone().unwrap().id, tex_idx);

    shared
        .ui
        .start_next_tutorial_step(TutorialStep::EditBoneX, &shared.armature);
}

pub fn read_psd(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context,
) {
    let psd: psd::Psd;
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !fs::exists(TEMP_IMPORT_PSD_PATH).unwrap() {
            return;
        }

        let psd_file_path = fs::read_to_string(TEMP_IMPORT_PSD_PATH).unwrap();
        let psd_file = std::fs::read(psd_file_path).unwrap();
        psd = psd::Psd::from_bytes(&psd_file).unwrap();
        del_temp_files();
    }

    #[cfg(target_arch = "wasm32")]
    {
        if getFile().len() == 0 || !getFileName().contains(".psd") {
            return;
        }
        psd = psd::Psd::from_bytes(&getFile()).unwrap();
        removeFile();
    }

    shared.ui.unselect_everything();

    // reset armature (but not all of it) to make way for the psd rig
    shared.armature.bones = vec![];
    shared.armature.bind_groups = vec![];
    shared.armature.textures = vec![];

    // collect group ids, to be used later
    let mut group_ids: Vec<u32> = vec![];
    for l in 0..psd.layers().len() {
        let layer = &psd.layers()[l];
        if layer.visible() || layer.parent_id() == None {
            continue;
        }
        if !group_ids.contains(&layer.parent_id().unwrap()) {
            group_ids.push(layer.parent_id().unwrap());
        }
    }

    let dimensions = Vec2::new(psd.width() as f32, psd.height() as f32);
    group_ids.reverse();
    for g in 0..group_ids.len() {
        let group = &psd.groups()[&group_ids[g]];

        // flatten this group's layers, to form the final texture
        let pixels = psd
            .flatten_layers_rgba(&|(_d, layer)| {
                if layer.parent_id() == None || layer.name().contains("$pivot") {
                    return false;
                }
                let parent_group = &psd.groups()[&layer.parent_id().unwrap()];
                parent_group.name().contains(group.name())
            })
            .unwrap();

        // get dimension and top left position of this group
        let mut dims = Vec2::default();
        let mut pos_tl = Vec2::new(f32::INFINITY, f32::INFINITY);
        for layer in psd.get_group_sub_layers(&group.id()).unwrap() {
            // ignore layers that aren't direct children of this group
            if layer.parent_id().unwrap() != group.id() {
                continue;
            }

            if layer.width() as f32 > dims.x {
                dims.x = layer.width() as f32;
            }
            if layer.height() as f32 > dims.y {
                dims.y = layer.height() as f32;
            }
            if (layer.layer_top() as f32) < pos_tl.y {
                pos_tl.y = layer.layer_top() as f32;
            }
            if (layer.layer_left() as f32) < pos_tl.x {
                pos_tl.x = layer.layer_left() as f32;
            }
        }

        // all layers use the full canvas size, so crop them
        let img_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(
            dimensions.x as u32,
            dimensions.y as u32,
            pixels.clone(),
        )
        .unwrap();
        let crop = image::imageops::crop_imm(
            &img_buf,
            pos_tl.x as u32,
            pos_tl.y as u32,
            dims.x as u32,
            dims.y as u32,
        )
        .to_image();

        add_texture(
            crop.to_vec(),
            dims,
            group.name(),
            &mut shared.ui,
            &mut shared.armature,
            queue,
            device,
            bind_group_layout,
            ctx,
        );

        // check if this group has a pivot, and create it if so
        let mut pivot_id = -1;
        let mut pivot_pos = Vec2::default();
        for l in 0..psd.layers().len() {
            let layer = &psd.layers()[l];
            if layer.parent_id() != Some(group_ids[g]) || !layer.name().contains("$pivot") {
                continue;
            }

            pivot_id = shared.armature.new_bone(-1).0.id;
            let pivot_bone = shared.armature.find_bone_mut(pivot_id).unwrap();
            pivot_pos = Vec2::new(layer.layer_left() as f32, -layer.layer_top() as f32);
            pivot_bone.pos = pivot_pos - Vec2::new(dimensions.x / 2., -dimensions.y / 2.);
            pivot_bone.name = group.name().to_string();
            pivot_bone.folded = true;
        }

        // create texture bone
        let new_bone_id = shared.armature.new_bone(-1).0.id;
        let tex_idx = shared.armature.textures.len() - 1;
        shared.armature.set_bone_tex(new_bone_id, tex_idx);
        let new_bone = shared.armature.find_bone_mut(new_bone_id).unwrap();

        // layers start from top-left, so push bone down and right to reflect that
        new_bone.pos = Vec2::new(dims.x / 2., -dims.y / 2.);

        // push bone to wherever it would have been on the canvas
        new_bone.pos.x += pos_tl.x;
        new_bone.pos.y -= pos_tl.y;

        // set up texture to be part of it's pivot, if it exists
        if pivot_id != -1 {
            new_bone.parent_id = pivot_id;
            new_bone.name = "Texture".to_string();

            new_bone.pos.x -= pivot_pos.x;
            new_bone.pos.y -= pivot_pos.y;
        } else {
            new_bone.pos -= Vec2::new(dimensions.x / 2., -dimensions.y / 2.);
        }
    }

    shared.ui.set_state(UiState::Modal, false);
}

pub fn add_texture(
    pixels: Vec<u8>,
    dimensions: Vec2,
    tex_name: &str,
    ui: &mut Ui,
    armature: &mut Armature,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context,
) {
    armature
        .bind_groups
        .push(renderer::create_texture_bind_group(
            pixels.clone(),
            dimensions,
            queue,
            device,
            bind_group_layout,
        ));

    let img_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(
        dimensions.x as u32,
        dimensions.y as u32,
        pixels.clone(),
    )
    .unwrap();

    ui.add_texture_img(&ctx, img_buf, Vec2::new(300., 300.));

    armature.textures.push(crate::Texture {
        offset: Vec2::ZERO,
        size: dimensions,
        pixels,
        name: tex_name.to_string(),
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_save(shared: &mut Shared) {
    if !fs::exists(TEMP_SAVE_PATH).unwrap() {
        return;
    }

    let path = fs::read_to_string(TEMP_SAVE_PATH).unwrap();

    shared.save_path = path.clone();

    utils::save(path, shared);

    del_temp_files();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_import(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    context: &egui::Context,
) {
    if !fs::exists(TEMP_IMPORT_PATH).unwrap() {
        return;
    }

    let path = fs::read_to_string(TEMP_IMPORT_PATH).unwrap();

    shared.save_path = path.clone();

    let file = std::fs::File::open(&path).unwrap();

    utils::import(
        &path,
        file,
        shared,
        queue,
        device,
        bind_group_layout,
        context,
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_exported_video_frame(shared: &mut Shared) {
    if !fs::exists(TEMP_EXPORT_VID_TEXT).unwrap() {
        return;
    }
    let frame = fs::read_to_string(TEMP_EXPORT_VID_TEXT).unwrap();
    shared.ui.open_modal(frame, false);
    fs::remove_file(TEMP_EXPORT_VID_TEXT).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn del_temp_files() {
    for f in FILES {
        if fs::exists(f).unwrap() {
            fs::remove_file(f).unwrap();
        }
    }
}

/// Load an iamge by reading an `img` tag with id `last-image`.
// Most code was generated by ChatGPT (sources unknown)
#[cfg(target_arch = "wasm32")]
pub fn load_image_wasm(id: String) -> Option<(Vec<u8>, Vec2)> {
    let mut result: Vec<u8> = vec![];
    let mut dimensions = Vec2::default();

    let document: Document = window().unwrap().document().unwrap();
    if let Some(img_element) = document.get_element_by_id(&id) {
        let img = img_element.dyn_into::<HtmlImageElement>().unwrap();

        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        canvas.set_width(img.width());
        canvas.set_height(img.height());

        // get 2D rendering context
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        if img.width() == 0 && img.height() == 0 {
            return None;
        }

        // draw image onto canvas
        context
            .draw_image_with_html_image_element(&img, 0.0, 0.0)
            .unwrap();

        // extract image data (RGBA pixels)
        let image_data = context
            .get_image_data(0.0, 0.0, img.width() as f64, img.height() as f64)
            .unwrap();
        let pixels = image_data.data();

        // convert js_sys::Uint8ClampedArray to Vec<u8>
        pixels.to_vec();

        dimensions = Vec2::new(img.width() as f32, img.height() as f32);
        result = pixels.to_vec();
    }

    return Some((result, dimensions));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn create_temp_file(name: &str, content: &str) {
    let mut img_path = std::fs::File::create(name).unwrap();
    img_path.write_all(content.as_bytes()).unwrap();
}

#[cfg(target_arch = "wasm32")]
pub fn load_file(
    shared: &mut crate::Shared,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
    bind_group_layout: &BindGroupLayout,
    context: &egui::Context,
) {
    if getFile().len() == 0 || !getFileName().contains(".skf") {
        return;
    }
    let cursor = std::io::Cursor::new(getFile());
    utils::import(
        "test.skf",
        cursor,
        shared,
        queue,
        device,
        bind_group_layout,
        context,
    );
    removeFile();
}
