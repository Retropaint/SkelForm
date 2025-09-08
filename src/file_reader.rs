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

    match renderer.as_ref() {
        None => return,
        _ => {}
    }

    func!(read_image_loaders);

    #[cfg(target_arch = "wasm32")]
    func!(load_file);

    #[cfg(not(target_arch = "wasm32"))]
    {
        read_save(shared);
        read_exported_video_frame(shared);
        func!(read_import);
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
    let image: image::DynamicImage;
    #[allow(unused_assignments)]
    let mut dimensions = Vec2::default();
    #[allow(unused_assignments, unused_mut)]
    let mut name = "".to_string();

    #[cfg(not(target_arch = "wasm32"))]
    {
        if !fs::exists(shared.temp_path.img.clone()).unwrap() {
            return;
        }

        // delete files if selected bone is invalid
        if shared.armature.bones.len() == 0
            || shared.ui.selected_bone_idx > shared.armature.bones.len() - 1
        {
            del_temp_files(&shared.temp_path.base);
            return;
        }

        let img_path = fs::read_to_string(shared.temp_path.img.clone()).unwrap();
        if img_path == "" {
            del_temp_files(&shared.temp_path.base);
            return;
        }

        // extract name
        let filename = img_path.split('/').last().unwrap().to_string();
        name = filename.split('.').collect::<Vec<_>>()[0].to_string();

        // read image pixels and dimensions
        let file_bytes = fs::read(img_path);
        image = image::load_from_memory(&file_bytes.unwrap()).unwrap();
        dimensions = Vec2::new(image.width() as f32, image.height() as f32);

        del_temp_files(&shared.temp_path.base);
    }

    #[cfg(target_arch = "wasm32")]
    {
        if let Some((wasm_pixels, dims)) = load_image_wasm("last-image".to_string()) {
            dimensions = Vec2::new(dims.x as f32, dims.y as f32);
            image = image::DynamicImage::ImageRgba8(
                image::ImageBuffer::from_raw(dims.x as u32, dims.y as u32, wasm_pixels).unwrap(),
            );
        } else {
            return;
        }

        name = getImgName().split('.').collect::<Vec<_>>()[0].to_string();

        removeImage();
    }

    if image.clone().into_rgba8().to_vec().len() == 0 {
        shared.ui.open_modal(IMPORT_IMG_ERR.to_string(), false);
        return;
    }

    shared.ui.set_state(UiState::ImageModal, false);

    // check if this texture already exists
    for set in &shared.armature.texture_sets {
        for tex in &set.textures {
            if image == tex.image {
                return;
            }
        }
    }

    add_texture(
        image,
        dimensions,
        &name,
        &mut shared.ui,
        &mut shared.armature,
        queue,
        device,
        bind_group_layout,
        ctx,
    );

    let mut anim_id = shared.ui.anim.selected;
    if !shared.ui.is_animating() {
        anim_id = usize::MAX;
        shared.undo_actions.push(Action {
            action: ActionEnum::Bone,
            id: shared.selected_bone().unwrap().id,
            bones: vec![shared.selected_bone().unwrap().clone()],
            ..Default::default()
        });
    } else {
        shared.undo_actions.push(Action {
            action: ActionEnum::Animation,
            id: shared.selected_animation().unwrap().id,
            animations: vec![shared.selected_animation().unwrap().clone()],
            ..Default::default()
        });
    }

    let tex_idx = shared.armature.texture_sets[shared.ui.selected_tex_set_idx as usize]
        .textures
        .len()
        - 1;
    shared.armature.set_bone_tex(
        shared.selected_bone().unwrap().id,
        tex_idx,
        shared.ui.selected_tex_set_idx,
        anim_id,
        shared.ui.anim.selected_frame,
    );
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
        let psd_file_path = fs::read_to_string(shared.temp_path.import.clone()).unwrap();
        let psd_file = std::fs::read(psd_file_path).unwrap();
        psd = psd::Psd::from_bytes(&psd_file).unwrap();
        del_temp_files(&shared.temp_path.base);
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
    shared.armature.texture_sets = vec![];

    // collect group ids, to be used later
    let mut group_ids: Vec<u32> = vec![];
    for l in 0..psd.layers().len() {
        let layer = &psd.layers()[l];
        // for some reason, layer.visible() is inverted
        if layer.visible() || layer.parent_id() == None {
            continue;
        }
        if !group_ids.contains(&layer.parent_id().unwrap()) {
            group_ids.push(layer.parent_id().unwrap());
        }
    }

    shared.armature.texture_sets.push(TextureSet {
        name: "Default".to_string(),
        textures: vec![],
    });

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
            image::DynamicImage::ImageRgba8(crop),
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
        let tex_idx = shared.armature.texture_sets[0].textures.len() - 1;
        shared.armature.set_bone_tex(
            new_bone_id,
            tex_idx,
            shared.ui.selected_tex_set_idx,
            shared.ui.anim.selected,
            shared.ui.anim.selected_frame,
        );
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

        let bone_id = if pivot_id != -1 {
            pivot_id
        } else {
            new_bone_id
        };

        if group.parent_id() == None {
            continue;
        }

        // find parent by name
        let parent_name = psd.groups()[&group.parent_id().unwrap()].name();
        for b in 0..shared.armature.bones.len() {
            if shared.armature.bones[b].name != parent_name {
                continue;
            }

            shared.armature.find_bone_mut(bone_id).unwrap().parent_id = shared.armature.bones[b].id;

            // since child pos is relative to parent, offset against it
            let mut nb = shared.armature.find_bone(bone_id).unwrap().clone();
            while nb.parent_id != -1 {
                nb = shared.armature.find_bone(nb.parent_id).unwrap().clone();
                shared.armature.find_bone_mut(bone_id).unwrap().pos -= nb.pos;
            }
            break;
        }
    }

    let str_psd = shared.loc("psd_imported");
    shared.ui.open_modal(str_psd.to_string(), false);
    shared.ui.set_state(UiState::StartupWindow, false);
}

pub fn add_texture(
    image: image::DynamicImage,
    dimensions: Vec2,
    tex_name: &str,
    ui: &mut Ui,
    armature: &mut Armature,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context,
) {
    let img_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(
        dimensions.x as u32,
        dimensions.y as u32,
        image.clone().into_rgba8().to_vec(),
    )
    .unwrap();

    let bind_group = renderer::create_texture_bind_group(
        image.clone().into_rgba8().to_vec(),
        dimensions,
        queue,
        device,
        bind_group_layout,
    );

    armature.texture_sets[ui.selected_tex_set_idx as usize]
        .textures
        .push(crate::Texture {
            offset: Vec2::ZERO,
            size: dimensions,
            image,
            name: tex_name.to_string(),
            bind_group: Some(bind_group),
            ui_img: Some(utils::add_texture_img(&ctx, img_buf, Vec2::new(300., 300.))),
        });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_save(shared: &mut Shared) {
    if !fs::exists(shared.temp_path.save.clone()).unwrap() {
        return;
    }

    let path = fs::read_to_string(shared.temp_path.save.clone()).unwrap();

    shared.save_path = path.clone();

    shared.saving = Saving::CustomPath;

    del_temp_files(&shared.temp_path.base);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_import(
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bgl: &BindGroupLayout,
    context: &egui::Context,
) {
    if !fs::exists(shared.temp_path.import.clone()).unwrap() {
        return;
    }

    let path = fs::read_to_string(shared.temp_path.import.clone()).unwrap();

    let file = std::fs::File::open(&path);

    if let Err(err) = file {
        println!("{}", err);
        del_temp_files(&shared.temp_path.base);
        return;
    }

    shared.save_path = path.clone();

    let ext = path.split('.').last().unwrap();
    match ext {
        "skf" => {
            utils::import(file.unwrap(), shared, queue, device, bgl, context);
            let full_path_canon = std::fs::canonicalize(&path);
            let full_path = if let Ok(_) = full_path_canon {
                &full_path_canon.unwrap().to_str().unwrap().to_string()
            } else {
                &path
            };
            if !shared.recent_file_paths.contains(&full_path) {
                shared.recent_file_paths.push(full_path.to_string());
            }
            utils::save_to_recent_files(&shared.recent_file_paths);
        }
        "psd" => read_psd(shared, queue, device, bgl, context),
        _ => {}
    };
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_exported_video_frame(shared: &mut Shared) {
    if !fs::exists(shared.temp_path.export_vid_text.clone()).unwrap() {
        return;
    }
    let frame = fs::read_to_string(shared.temp_path.export_vid_text.clone()).unwrap();
    shared.ui.open_modal(frame, false);
    fs::remove_file(shared.temp_path.export_vid_text.clone()).unwrap();
}

pub fn del_temp_files(_base: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        for file in glob::glob(&(_base.to_string() + "*")).unwrap() {
            if let Ok(path) = file {
                match fs::remove_file(path) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("{}", e)
                    }
                }
            }
        }
    }
}

/// Load image by reading an `img` tag with id `last-image`.
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
    utils::import(cursor, shared, queue, device, bind_group_layout, context);
    removeFile();
}
