//! Reading uploaded images to turn into textures.
// test

use std::sync::Mutex;

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
                Some(&renderer.as_ref().unwrap().gpu.queue),
                Some(&renderer.as_ref().unwrap().gpu.device),
                Some(&renderer.as_ref().unwrap().bind_group_layout),
                Some(&context),
            )
        };
    }

    match renderer.as_ref() {
        None => return,
        _ => {}
    }

    func!(read_image_loaders);
    func!(read_import);
}
/// read temporary files created from file dialogs (native & WASM)
pub fn read_image_loaders(
    shared: &mut Shared,
    queue: Option<&Queue>,
    device: Option<&Device>,
    bind_group_layout: Option<&BindGroupLayout>,
    ctx: Option<&egui::Context>,
) {
    let image: image::DynamicImage;
    #[allow(unused_assignments)]
    let mut dimensions = Vec2::default();
    #[allow(unused_assignments, unused_mut)]
    let mut name = "".to_string();

    #[cfg(not(target_arch = "wasm32"))]
    {
        if shared.img_contents.lock().unwrap().len() == 0 {
            return;
        }

        // extract name
        let raw_filename = shared.file_name.lock().unwrap();
        let filename = raw_filename.split('/').last().unwrap().to_string();
        name = filename.split('.').collect::<Vec<_>>()[0].to_string();

        // read image pixels and dimensions
        image = image::load_from_memory(&shared.img_contents.lock().unwrap()).unwrap();
        dimensions = Vec2::new(image.width() as f32, image.height() as f32);

        *shared.img_contents.lock().unwrap() = vec![];
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
    queue: Option<&Queue>,
    device: Option<&Device>,
    bind_group_layout: Option<&BindGroupLayout>,
    ctx: Option<&egui::Context>,
) {
    let psd = psd::Psd::from_bytes(&bytes).unwrap();

    // reset armature (but not all of it) to make way for the psd rig
    shared.armature.bones = vec![];
    shared.armature.styles = vec![];

    // create root bone, where all except targets will go
    shared.armature.new_bone(-1);
    shared.armature.bones[0].name = "Root".to_string();
    shared.armature.bones[0].folded = true;

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
    let _images: Arc<Mutex<Vec<ImageType>>> = Arc::new(vec![].into());

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut processes = vec![];
        let cgroup_ids = group_ids.clone();
        let cimages = Arc::clone(&_images);
        processes.push(std::thread::spawn(move || {
            for g in (0..cgroup_ids.len()).rev() {
                let psd = psd::Psd::from_bytes(&bytes).unwrap();
                let group = psd.groups()[&cgroup_ids[g]].clone();
                cimages.lock().unwrap().push(load_psd_tex(psd, group));
            }
        }));
        for process in processes {
            process.join().unwrap();
        }
    }

    group_ids.reverse();
    for g in 0..group_ids.len() {
        let group = &psd.groups()[&group_ids[g]];
        let image: (image::ImageBuffer<Rgba<u8>, Vec<u8>>, Vec2);

        #[cfg(not(target_arch = "wasm32"))]
        {
            image = _images.lock().unwrap()[g].clone();
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
                utils::without_unicode(tex_name),
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
            pivot_bone.parent_id = 0;
            pivot_bone.pos = pivot_pos - Vec2::new(dimensions.x / 2., -dimensions.y / 2.);
            pivot_bone.name = utils::without_unicode(group.name()).to_string();
            pivot_bone.folded = true;
            pivot_bone.zindex = 0;
        }

        // create texture bone
        let new_bone_id = shared.armature.new_bone(-1).0.id;
        if pivot_id == -1 {
            bone_psd_id.insert(new_bone_id, group_ids[g] as u32);
        }
        let tex_name = shared.armature.styles[0].textures[tex_idx].name.clone();
        let bone = shared.armature.find_bone_mut(new_bone_id).unwrap();
        bone.parent_id = 0;
        bone.style_ids = vec![0];
        shared
            .armature
            .set_bone_tex(new_bone_id, tex_name.clone(), usize::MAX, 0);

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
                bone.ik_constraint = JointConstraint::CounterClockwise;
            } else if layer.name().contains("clockwise") {
                bone.ik_constraint = JointConstraint::Clockwise;
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
            let bones = &shared.armature.bones;
            let mut nb = bones.iter().find(|bo| bo.id == bone_id).unwrap().clone();
            while nb.parent_id != -1 {
                let bones = &shared.armature.bones;
                let id = nb.parent_id;
                nb = bones.iter().find(|bo| bo.id == id).unwrap().clone();
                shared.armature.find_bone_mut(bone_id).unwrap().pos -= nb.pos;
            }
            break;
        }
    }

    // add IK targets
    for eff_id in start_eff_ids {
        let target_id = shared.armature.new_bone(-1).0.id;
        let start_eff_bone = &mut shared.armature.find_bone_mut(eff_id).unwrap();
        let ik_id = start_eff_bone.ik_family_id;

        start_eff_bone.ik_target_id = target_id;
        let target_name = start_eff_bone.name.to_owned() + " Target";

        // determine target's base position
        let effs = shared
            .armature
            .bones
            .iter()
            .filter(|bone| bone.ik_family_id == ik_id);
        let mut pos = effs.clone().last().unwrap().pos;
        let parents = shared.armature.get_all_parents(effs.last().unwrap().id);
        for bone in parents {
            pos += bone.pos;
        }

        let target_bone = shared.armature.find_bone_mut(target_id).unwrap();
        target_bone.name = target_name;
        target_bone.pos = pos;
        target_bone.zindex = 0;
    }

    let str_psd = &shared.loc("psd_imported");
    shared.ui.open_modal(str_psd.to_string(), false);
    shared.ui.startup_window = false;
}

/// add texture to style, including it's bind group and UI image.
pub fn add_texture(
    image: image::DynamicImage,
    style_id: i32,
    dimensions: Vec2,
    tex_name: &str,
    armature: &mut Armature,
    queue: Option<&Queue>,
    device: Option<&Device>,
    bind_group_layout: Option<&BindGroupLayout>,
    ctx: Option<&egui::Context>,
) {
    let img_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(
        dimensions.x as u32,
        dimensions.y as u32,
        image.clone().into_rgba8().to_vec(),
    )
    .unwrap();

    let mut bind_group = None;

    if queue != None && device != None && bind_group_layout != None {
        bind_group = Some(renderer::create_texture_bind_group(
            image.clone().into_rgba8().to_vec(),
            dimensions,
            queue.unwrap(),
            device.unwrap(),
            bind_group_layout.unwrap(),
        ));
    }

    let mut ui_img = None;
    if ctx != None {
        ui_img = Some(utils::add_texture_img(
            &ctx.unwrap(),
            img_buf,
            Vec2::new(300., 300.),
        ));
    }

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
            bind_group,
            ui_img,
            ser_offset: Vec2I::new(0, 0),
            ser_size: Vec2I::new(0, 0),
        });
}

pub fn read_import(
    shared: &mut Shared,
    queue: Option<&Queue>,
    device: Option<&Device>,
    bgl: Option<&BindGroupLayout>,
    context: Option<&egui::Context>,
) {
    let filename;
    let file;

    #[cfg(not(target_arch = "wasm32"))]
    {
        filename = shared.file_name.lock().unwrap().to_string();
        if shared.import_contents.lock().unwrap().len() == 0 {
            return;
        }
        *shared.import_contents.lock().unwrap() = vec![];

        file = std::fs::File::open(shared.file_name.lock().unwrap().to_string());
        if let Err(err) = file {
            let text = shared.loc("import_err").to_owned() + &err.to_string();
            shared.ui.open_modal(text.to_string(), false);
            return;
        }

        shared.save_path = filename.clone();
    }

    #[cfg(target_arch = "wasm32")]
    {
        filename = getFileName();
        if filename == "" {
            return;
        }
        file = getFile();
    }

    let ext = filename.split('.').last().unwrap();
    match ext {
        "skf" => {
            #[cfg(target_arch = "wasm32")]
            {
                let cursor = std::io::Cursor::new(getFile());
                utils::import(cursor, shared, queue, device, bgl, context);
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                utils::import(file.unwrap(), shared, queue, device, bgl, context);
                if !shared.recent_file_paths.contains(&filename) {
                    shared.recent_file_paths.push(filename);
                }
                utils::save_to_recent_files(&shared.recent_file_paths);
            }
        }
        "psd" => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let file = std::fs::read(filename).unwrap();
                read_psd(file, shared, queue, device, bgl, context);
            }
            #[cfg(target_arch = "wasm32")]
            read_psd(file, shared, queue, device, bgl, context)
        }
        _ => {
            let text = &shared.loc("import_unrecognized");
            shared.ui.open_modal(text.to_string(), false);
        }
    };

    *shared.import_contents.lock().unwrap() = vec![];
    #[cfg(target_arch = "wasm32")]
    removeFile();
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
