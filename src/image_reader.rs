//! Reading uploaded images to turn into textures.

use wgpu::*;

use crate::*;

/// read temporary files created from file dialogs (native & WASM)
pub fn read_image_loaders(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
) {
    #[allow(unused_assignments)]
    let mut pixels: Vec<u8> = vec![];
    #[allow(unused_assignments)]
    let mut dimensions: Vec2 = vec2! {0., 0.};

    #[cfg(not(target_arch = "wasm32"))]
    {
        if !fs::exists(".skelform_img_path").unwrap() {
            return;
        }

        // delete files if selected bone is invalid
        if shared.armature.bones.len() == 0
            || shared.selected_bone > shared.armature.bones.len() - 1
        {
            del_temp_files();
            return;
        }

        let img_path = fs::read_to_string(".skelform_img_path").unwrap();
        if img_path == "" {
            del_temp_files();
            return;
        }

        // read image pixels and dimensions
        let file_bytes = fs::read(img_path);
        let diffuse_image = image::load_from_memory(&file_bytes.unwrap()).unwrap();
        let rgba = diffuse_image.to_rgba8();
        pixels = rgba.as_bytes().to_vec();
        dimensions = vec2![diffuse_image.width() as f32, diffuse_image.height() as f32];

        del_temp_files();
    }

    #[cfg(target_arch = "wasm32")]
    {
        if let Some((wasm_pixels, dims)) = utils::load_image_wasm() {
            dimensions = vec2!(dims.x as f32, dims.y as f32);
            pixels = wasm_pixels;
        }

        removeImage();
    }

    if pixels.len() == 0 {
        return;
    }

    // add this texture to bind_groups array
    shared.bind_groups.push(renderer::create_texture(
        pixels.to_vec(),
        dimensions,
        &mut shared.armature.textures,
        queue,
        device,
        bind_group_layout,
    ));

    // assign this texture to the selected bone
    shared.armature.bones[shared.selected_bone].tex_idx = shared.armature.textures.len() - 1;
}

#[cfg(not(target_arch = "wasm32"))]
fn del_temp_files() {
    #[rustfmt::skip]
    let files = [
        ".skelform_img_path", 
        ".skelform_bone_idx"
    ];
    for f in files {
        if fs::exists(f).unwrap() {
            fs::remove_file(f).unwrap();
        }
    }
}
