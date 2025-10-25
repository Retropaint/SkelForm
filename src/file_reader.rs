//! Reading uploaded images to turn into textures.
// test

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
use wgpu::*;

use crate::*;
use image::Rgba;

use image::ImageBuffer;

// web-only imports
#[cfg(target_arch = "wasm32")]
mod web {
    pub use std::io::Read;
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
        read_save_finish(shared);
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

    // check if this texture already exists
    for tex in &shared.selected_set().unwrap().textures {
        if image == tex.image {
            return;
        }
    }

    add_texture(
        image,
        shared.ui.selected_tex_set_id,
        dimensions,
        &name,
        &mut shared.armature,
        queue,
        device,
        bind_group_layout,
        ctx,
    );
}

pub fn read_psd(
    bytes: Vec<u8>,
    shared: &mut Shared,
    queue: &Queue,
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    ctx: &egui::Context,
) {
    let psd = psd::Psd::from_bytes(&bytes).unwrap();
    del_temp_files(&shared.temp_path.base);

    // reset armature (but not all of it) to make way for the psd rig
    shared.armature.bones = vec![];
    shared.armature.styles = vec![];

    // collect group ids, to be used later
    let mut group_ids: Vec<u32> = vec![];
    for l in 0..psd.layers().len() {
        let layer = &psd.layers()[l];
        if !layer.visible() || layer.parent_id() == None {
            continue;
        }
        if !group_ids.contains(&layer.parent_id().unwrap()) {
            group_ids.push(layer.parent_id().unwrap());
        }
    }

    shared.armature.styles.push(Style {
        id: 0,
        name: "Default".to_string(),
        textures: vec![],
        active: true,
    });

    let mut bone_psd_id: std::collections::HashMap<i32, u32> = Default::default();
    let mut start_eff_ids: Vec<i32> = vec![];
    let mut ik_family_ids: Vec<i32> = vec![];
    let dimensions = Vec2::new(psd.width() as f32, psd.height() as f32);

    type ImageType = (ImageBuffer<Rgba<u8>, Vec<u8>>, Vec2);
    let mut images: Vec<ImageType> = vec![];

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut processes = vec![];
        async_std::task::block_on(async {
            for g in (0..group_ids.len()).rev() {
                let group = psd.groups()[&group_ids[g]].clone();
                let psd = psd::Psd::from_bytes(&bytes).unwrap();
                processes.push(async_std::task::spawn(load_psd_tex_async(psd, group)));
            }
            for process in processes {
                images.push(process.await);
            }
        });
    }

    group_ids.reverse();
    for g in 0..group_ids.len() {
        let group = &psd.groups()[&group_ids[g]];
        let image: (image::ImageBuffer<Rgba<u8>, Vec<u8>>, Vec2);

        #[cfg(not(target_arch = "wasm32"))]
        {
            image = images[g].clone();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let cpsd = psd::Psd::from_bytes(&bytes).unwrap();
            let cgroup = group.clone();
            image = load_psd_tex(cpsd, cgroup.clone());
        }

        let dims = Vec2::new(image.0.width() as f32, image.0.height() as f32);

        // add tex if not a duplicate
        let mut tex_idx = usize::MAX;
        for t in 0..shared.armature.styles[0].textures.len() {
            let img = &shared.armature.styles[0].textures[t].image;
            if img.to_rgba8().to_vec() == image.0.to_vec() {
                tex_idx = t;
                break;
            }
        }
        if tex_idx == usize::MAX {
            let mut style_idx: i32 = 0;
            let tex_name = group.name();

            if group.name().contains("$style") {
                let style_name =
                    utils::without_unicode(utils::after_underscore(group.name())).to_string();
                let styles = &shared.armature.styles;
                let names: Vec<String> = styles.iter().map(|style| style.name.clone()).collect();
                if let Some(idx) = names
                    .iter()
                    .position(|name| name.to_lowercase() == style_name.to_lowercase())
                {
                    style_idx = idx as i32;
                } else {
                    let new_idx = shared.armature.styles.len() as i32;
                    shared.armature.styles.push(Style {
                        id: shared.armature.styles.len() as i32,
                        name: style_name.to_string(),
                        active: true,
                        textures: vec![],
                    });
                    style_idx = new_idx;
                }
            }

            add_texture(
                image::DynamicImage::ImageRgba8(image.0.clone()),
                style_idx,
                Vec2::new(dims.x, dims.y),
                tex_name,
                &mut shared.armature,
                queue,
                device,
                bind_group_layout,
                ctx,
            );

            tex_idx = shared.armature.styles[0].textures.len() - 1;
        }

        if group.name().contains("$style") {
            continue;
        }

        // check if this group has a pivot, and create it if so
        let mut pivot_id = -1;
        let mut pivot_pos = Vec2::default();
        for l in 0..psd.layers().len() {
            let layer = &psd.layers()[l];
            if layer.parent_id() != Some(group_ids[g]) || !layer.name().contains("$pivot") {
                continue;
            }

            pivot_id = shared.armature.new_bone(-1).0.id;
            bone_psd_id.insert(pivot_id, group_ids[g] as u32);
            let pivot_bone = shared.armature.find_bone_mut(pivot_id).unwrap();
            pivot_pos = Vec2::new(layer.layer_left() as f32, -layer.layer_top() as f32);
            pivot_bone.pos = pivot_pos - Vec2::new(dimensions.x / 2., -dimensions.y / 2.);
            pivot_bone.name = group.name().to_string();
            pivot_bone.folded = true;
        }

        // create texture bone
        let new_bone_id = shared.armature.new_bone(-1).0.id;
        if pivot_id == -1 {
            bone_psd_id.insert(new_bone_id, group_ids[g] as u32);
        }
        let tex_name = shared.armature.styles[0].textures[tex_idx].name.clone();
        let bone = shared.armature.find_bone_mut(new_bone_id).unwrap();
        bone.style_ids = vec![0];
        shared.armature.set_bone_tex(
            new_bone_id,
            tex_idx,
            shared.ui.anim.selected,
            shared.ui.anim.selected_frame,
        );

        // process inverse kinematics layers ($ik_)
        for l in 0..psd.layers().len() {
            let layer = &psd.layers()[l];
            if layer.parent_id() != Some(group_ids[g]) || !layer.name().contains("$ik_") {
                continue;
            }

            let bone;
            if pivot_id != -1 {
                bone = shared.armature.find_bone_mut(pivot_id).unwrap();
            } else {
                bone = shared.armature.find_bone_mut(new_bone_id).unwrap();
            }

            if layer.name().contains("counterclockwise") {
                bone.constraint = JointConstraint::CounterClockwise;
            } else if layer.name().contains("clockwise") {
                bone.constraint = JointConstraint::Clockwise;
            } else {
                let num = utils::without_unicode(utils::after_underscore(layer.name()));
                match num.parse::<i32>() {
                    Ok(id) => {
                        bone.ik_family_id = id;
                        if !ik_family_ids.contains(&id) {
                            start_eff_ids.push(bone.id);
                            ik_family_ids.push(id);
                        }
                    }
                    Err(err) => println!("{}", err),
                }
            }
        }

        let new_bone = shared.armature.find_bone_mut(new_bone_id).unwrap();
        new_bone.name = tex_name;

        // layers start from top-left, so push bone down and right to reflect that
        new_bone.pos = Vec2::new(dims.x / 2., -dims.y / 2.);

        // push bone to wherever it would have been on the canvas
        new_bone.pos.x += image.1.x;
        new_bone.pos.y -= image.1.y;

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

        // find parent by group id
        for b in 0..shared.armature.bones.len() {
            let id = shared.armature.bones[b].id;
            let psd_id = bone_psd_id.get(&id);
            if psd_id == None || group.parent_id().unwrap() != *psd_id.unwrap() {
                continue;
            }

            shared.armature.find_bone_mut(bone_id).unwrap().parent_id = shared.armature.bones[b].id;
            shared.armature.bones[b].folded = true;

            // since child pos is relative to parent, offset against it
            let mut nb = shared.armature.find_bone(bone_id).unwrap().clone();
            while nb.parent_id != -1 {
                nb = shared.armature.find_bone(nb.parent_id).unwrap().clone();
                shared.armature.find_bone_mut(bone_id).unwrap().pos -= nb.pos;
            }
            break;
        }
    }

    // add IK targets
    for eff_id in start_eff_ids {
        let target_id = shared.armature.new_bone(-1).0.id;
        shared.armature.find_bone_mut(eff_id).unwrap().ik_target_id = target_id;
        let target_name = shared.armature.find_bone(eff_id).unwrap().name.to_owned() + " Target";
        shared.armature.find_bone_mut(target_id).unwrap().name = target_name;
    }

    let str_psd = shared.loc("psd_imported");
    shared.ui.open_modal(str_psd.to_string(), false);
    shared.ui.set_state(UiState::StartupWindow, false);
}

pub fn add_texture(
    image: image::DynamicImage,
    style_id: i32,
    dimensions: Vec2,
    tex_name: &str,
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

    armature
        .styles
        .iter_mut()
        .find(|set| set.id == style_id)
        .unwrap()
        .textures
        .push(crate::Texture {
            offset: Vec2::ZERO,
            size: dimensions,
            image,
            name: tex_name.to_string(),
            bind_group: Some(bind_group),
            ui_img: Some(utils::add_texture_img(&ctx, img_buf, Vec2::new(300., 300.))),
            ser_offset: Vec2I::new(0, 0),
            ser_size: Vec2I::new(0, 0),
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
pub fn read_save_finish(shared: &mut Shared) {
    if !fs::exists(shared.temp_path.save_finish.clone()).unwrap() {
        return;
    }

    shared.ui.set_state(UiState::Modal, false);

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
        let text = shared.loc("import_err").to_owned() + &err.to_string();
        shared.ui.open_modal(text.to_string(), false);
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
        "psd" => {
            let psd_file_path = fs::read_to_string(shared.temp_path.import.clone()).unwrap();
            let psd_file = std::fs::read(psd_file_path).unwrap();
            read_psd(psd_file, shared, queue, device, bgl, context)
        }
        _ => {
            let text = shared.loc("import_unrecognized");
            shared.ui.open_modal(text.to_string(), false);
            del_temp_files(&shared.temp_path.base);
        }
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
    let is_skf = getFileName().contains(".skf");
    let is_psd = getFileName().contains(".psd");
    if getFile().len() == 0 || (!is_psd && !is_skf) {
        return;
    }

    let cursor = std::io::Cursor::new(getFile());
    if is_psd {
        read_psd(
            cursor.into_inner(),
            shared,
            queue,
            device,
            bind_group_layout,
            context,
        );
    } else if is_skf {
        utils::import(cursor, shared, queue, device, bind_group_layout, context);
    }

    removeFile();
}

#[cfg(target_arch = "wasm32")]
fn load_psd_tex(
    psd: psd::Psd,
    group: psd::PsdGroup,
) -> (image::ImageBuffer<Rgba<u8>, Vec<u8>>, Vec2) {
    let (pixels, width, height, tl_x, tl_y) = psd
        .flatten_layers_rgba(&|(_d, layer)| {
            if layer.parent_id() == None || layer.name().contains("$") {
                return false;
            }
            let parent_group = &psd.groups()[&layer.parent_id().unwrap()];
            parent_group.id() == group.id()
        })
        .unwrap();

    let img_buf =
        <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(width, height, pixels).unwrap();

    (img_buf, Vec2::new(tl_x as f32, tl_y as f32))
}

async fn load_psd_tex_async(
    psd: psd::Psd,
    group: psd::PsdGroup,
) -> (image::ImageBuffer<Rgba<u8>, Vec<u8>>, Vec2) {
    let (pixels, width, height, tl_x, tl_y) = psd
        .flatten_layers_rgba(&|(_d, layer)| {
            if layer.parent_id() == None || layer.name().contains("$") {
                return false;
            }
            let parent_group = &psd.groups()[&layer.parent_id().unwrap()];
            parent_group.id() == group.id()
        })
        .unwrap();

    let img_buf =
        <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(width, height, pixels).unwrap();

    (img_buf, Vec2::new(tl_x as f32, tl_y as f32))
}
