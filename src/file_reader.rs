//! Reading uploaded images to turn into textures.
// test

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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn removeImage();
}

pub const TEMP_IMG_PATH: &str = ".skelform_img_path";
pub const TEMP_EXPORT_PATH: &str = ".skelform_export_path";
pub const TEMP_IMPORT_PATH: &str = ".skelform_import_path";
pub const TEMP_EXPORT_VID_TEXT: &str = ".skelform_export_video_text";

pub const FILES: [&str; 4] = [
    TEMP_IMG_PATH,
    TEMP_EXPORT_PATH,
    TEMP_IMPORT_PATH,
    TEMP_EXPORT_VID_TEXT,
];

pub const EXPORT_VID_DONE: &str = "Done!";

pub fn read(shared: &mut Shared, renderer: &Option<Renderer>, context: &egui::Context) {
    if let Some(_) = renderer.as_ref() {
        file_reader::read_image_loaders(
            shared,
            &renderer.as_ref().unwrap().gpu.queue,
            &renderer.as_ref().unwrap().gpu.device,
            &renderer.as_ref().unwrap().bind_group_layout,
            context,
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
            file_reader::read_export(&shared);
            file_reader::read_import(
                shared,
                &renderer.as_ref().unwrap().gpu.queue,
                &renderer.as_ref().unwrap().gpu.device,
                &renderer.as_ref().unwrap().bind_group_layout,
            );
            file_reader::read_exported_video_frame(shared);
        }
    }
}
/// read temporary files created from file dialogs (native & WASM)
pub fn read_image_loaders(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context
) {
    #[allow(unused_assignments)]
    let mut pixels: Vec<u8> = vec![];
    #[allow(unused_assignments)]
    let mut dimensions: Vec2 = Vec2::new(0., 0.);

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
        if let Some((wasm_pixels, dims)) = load_image_wasm() {
            dimensions = Vec2::new(dims.x as f32, dims.y as f32);
            pixels = wasm_pixels;
        }

        removeImage();
    }

    if pixels.len() == 0 {
        return;
    }

    // check if this texture already exists
    for tex in &shared.armature.textures {
        if pixels == tex.pixels {
            return;
        }
    }

    // add this texture to bind_groups array
    shared.bind_groups.push(renderer::create_texture_bind_group(
        pixels.to_vec(),
        dimensions,
        queue,
        device,
        bind_group_layout,
    ));

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [
            dimensions.x as usize,
            dimensions.y as usize,
        ],
        &pixels,
    );
    let tex = ctx.load_texture("anim_icons", color_image, Default::default());
    shared.ui.texture_images.push(tex);

    shared.armature.textures.push(crate::Texture {
        size: dimensions,
        pixels,
    });

    // assign this texture to the selected bone
    shared.armature.bones[shared.selected_bone_idx].tex_idx =
        shared.armature.textures.len() as i32 - 1;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_export(shared: &Shared) {
    if !fs::exists(TEMP_EXPORT_PATH).unwrap() {
        return;
    }

    let path = fs::read_to_string(TEMP_EXPORT_PATH).unwrap();

    utils::export(path, &shared.armature.textures, &shared.armature);

    del_temp_files();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_import(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
) {
    if !fs::exists(TEMP_IMPORT_PATH).unwrap() {
        return;
    }

    let path = fs::read_to_string(TEMP_IMPORT_PATH).unwrap();

    utils::import(path, shared, queue, device, bind_group_layout);

    del_temp_files();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_exported_video_frame(shared: &mut Shared) {
    if !fs::exists(TEMP_EXPORT_VID_TEXT).unwrap() {
        return;
    }
    let frame = fs::read_to_string(TEMP_EXPORT_VID_TEXT).unwrap();
    shared.ui.modal_headline = frame;
    shared.ui.forced_modal = shared.ui.modal_headline != EXPORT_VID_DONE;
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
pub fn load_image_wasm() -> Option<(Vec<u8>, Vec2)> {
    let mut result: Vec<u8> = vec![];
    let mut dimensions = Vec2::default();

    let document: Document = window().unwrap().document().unwrap();
    if let Some(img_element) = document.get_element_by_id("last-image") {
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

        log::info!("image!");

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
