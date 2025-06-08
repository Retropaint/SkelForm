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
#[rustfmt::skip] temp_file!(TEMP_IMPORT_TIFF_PATH, ".skelform_import_tiff_path");

pub const FILES: [&str; 5] = [
    TEMP_IMG_PATH,
    TEMP_SAVE_PATH,
    TEMP_IMPORT_PATH,
    TEMP_IMPORT_TIFF_PATH,
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
            || shared.selected_bone_idx > shared.armature.bones.len() - 1
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
        shared,
        queue,
        device,
        bind_group_layout,
        ctx,
    );

    let tex_idx = shared.armature.textures.len() - 1;
    shared.set_bone_tex(shared.selected_bone().unwrap().id, tex_idx);

    shared.start_next_tutorial_step(TutorialStep::EditBoneX);
}

pub fn read_psd(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context,
) {
    if !fs::exists(TEMP_IMPORT_TIFF_PATH).unwrap() {
        return;
    }

    shared.unselect_everything();
    shared.armature = Armature::default();

    let psd_file_path = fs::read_to_string(TEMP_IMPORT_TIFF_PATH).unwrap();

    let psd_file = std::fs::read(psd_file_path).unwrap();
    let psd = psd::Psd::from_bytes(&psd_file).unwrap();

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
        let group_name = psd.groups()[&group_ids[g]].name();
        let pixels = psd
            .flatten_layers_rgba(&|(_d, layer)| {
                if layer.parent_id() == None || layer.name().contains("$pivot") {
                    return false;
                }
                let group = &psd.groups()[&layer.parent_id().unwrap()];
                group.name().contains(group_name)
            })
            .unwrap();

        add_texture(
            pixels,
            dimensions,
            group_name,
            shared,
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

            pivot_id = armature_window::new_bone(shared, -1).0.id;
            let center = dimensions / 2.;
            pivot_pos = Vec2::new(
                layer.layer_left() as f32 - center.x,
                -(layer.layer_top() as f32 - center.y),
            );
            shared.find_bone_mut(pivot_id).unwrap().pos = pivot_pos;
            shared.find_bone_mut(pivot_id).unwrap().name = group_name.to_string();
            shared.find_bone_mut(pivot_id).unwrap().folded = true;
        }

        // create texture bone
        let new_bone_id = armature_window::new_bone(shared, -1).0.id;
        let tex_idx = shared.armature.textures.len() - 1;
        shared.set_bone_tex(new_bone_id, tex_idx);

        // set up texture to be part of it's pivot, if it exists
        if pivot_id != -1 {
            shared.find_bone_mut(new_bone_id).unwrap().parent_id = pivot_id;
            shared.find_bone_mut(new_bone_id).unwrap().name = "Texture".to_string();

            // offset texture against pivot, so it ends back at origin of it's canvas
            shared.find_bone_mut(new_bone_id).unwrap().pos -= pivot_pos;
        }
    }

    shared.ui.set_state(UiState::Modal, false);

    del_temp_files();
}

pub fn add_texture(
    pixels: Vec<u8>,
    dimensions: Vec2,
    tex_name: &str,
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context,
) {
    shared.bind_groups.push(renderer::create_texture_bind_group(
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
    let resized = image::imageops::resize(&img_buf, 300, 300, imageops::FilterType::Nearest);

    let color_image = egui::ColorImage::from_rgba_unmultiplied([300, 300], &resized);
    let tex = ctx.load_texture("anim_icons", color_image, Default::default());
    shared.ui.texture_images.push(tex);

    shared.armature.textures.push(crate::Texture {
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
    if getFile().len() == 0 {
        return;
    }
    let cursor = std::io::Cursor::new(getFile());
    utils::import(cursor, shared, queue, device, bind_group_layout, context);
    removeFile();
}
