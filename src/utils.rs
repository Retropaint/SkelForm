//! Isolated set of helper functions.

use crate::*;

#[cfg(target_arch = "wasm32")]
mod web {
    pub use wasm_bindgen::prelude::wasm_bindgen;
    pub use web_sys::*;
    pub use zip::write::FileOptions;
}
use max_rects::packing_box::PackingBox;
use renderer::construction;
#[cfg(target_arch = "wasm32")]
pub use web::*;

use image::{ExtendedColorType::Rgb8, GenericImage, ImageEncoder};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
use std::{collections::HashMap, path::PathBuf};

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
pub fn open_save_dialog(file_path: &Arc<Mutex<Vec<PathBuf>>>, saving: &Arc<Mutex<Saving>>) {
    let filepath = Arc::clone(&file_path);
    let csaving = Arc::clone(&saving);
    std::thread::spawn(move || {
        let fil = "SkelForm Armature";
        let task = rfd::FileDialog::new().add_filter(fil, &["skf"]).save_file();
        if task == None {
            return;
        }
        *filepath.lock().unwrap() = vec![task.unwrap()];
        *csaving.lock().unwrap() = shared::Saving::CustomPath;
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_import_dialog(file_path: &Arc<Mutex<Vec<PathBuf>>>, file_type: &Arc<Mutex<i32>>) {
    let filepath = Arc::clone(&file_path);
    let filetype = Arc::clone(&file_type);
    std::thread::spawn(move || {
        let task = rfd::FileDialog::new().pick_file();
        if task == None {
            return;
        }
        *filepath.lock().unwrap() = vec![task.unwrap()];
        *filetype.lock().unwrap() = 2;
    });
}

#[cfg(target_arch = "wasm32")]
pub fn save_web(
    armature: &Armature,
    camera: &Camera,
    selection: &SelectionState,
    edit_mode: &EditMode,
) {
    let mut png_bufs = vec![];
    let mut sizes = vec![];
    let mut carmature = armature.clone();

    if carmature.styles.len() > 0 && carmature.styles[0].textures.len() > 0 {
        (png_bufs, sizes) = utils::create_tex_sheet(&mut carmature);
    }

    let (armatures_json, editor_json) = prepare_files(&carmature, camera.clone(), sizes.clone());

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
    zip.start_file("readme.md", options.clone()).unwrap();
    zip.write(include_bytes!("../assets/skf_readme.md"))
        .unwrap();
    for i in 0..png_bufs.len() {
        let name = "atlas".to_owned() + &i.to_string() + ".png";
        zip.start_file(name, options.clone()).unwrap();
        zip.write(&png_bufs[i]).unwrap();
    }

    let bytes = zip.finish().unwrap().into_inner().to_vec();
    downloadZip(bytes);
}

pub fn create_tex_sheet(armature: &mut Armature) -> (Vec<Vec<u8>>, Vec<i32>) {
    let mut atlases: Vec<Vec<PackingBox>> = vec![];
    let mut sizes: Vec<i32> = vec![];
    let mut boxes = vec![];
    let max = 2048;

    atlases.push(vec![]);
    sizes.push(0);

    for s in 0..armature.styles.len() {
        let mut style_boxes = vec![];
        for t in 0..armature.styles[s].textures.len() {
            let tex = &armature.styles[s].textures[t];
            let image = &armature.tex_data(tex).unwrap().image;
            boxes.push(PackingBox::new(image.width() as i32, image.height() as i32));
            style_boxes.push(PackingBox::new(image.width() as i32, image.height() as i32));
        }

        'atlas_maker: loop {
            let first_style_in_atlas = *sizes.last().unwrap() == 0;
            let og_size = sizes.last().unwrap().clone();
            let new_atlas;
            loop {
                // if this is the first style in the atlas, ignore max limit
                if *sizes.last().unwrap() >= max && !first_style_in_atlas {
                    new_atlas = true;
                    break;
                }
                *sizes.last_mut().unwrap() += 128;
                let size = *sizes.last().unwrap();
                let bins = vec![max_rects::bucket::Bucket::new(size - 1, size - 1, 0, 0, 1)];
                let mut problem = max_rects::max_rects::MaxRects::new(boxes.clone(), bins.clone());
                let (placed, _, _) = problem.place();
                if placed.len() == boxes.len() {
                    for tex in &mut armature.styles[s].textures {
                        tex.atlas_idx = atlases.len() as i32 - 1;
                    }
                    *atlases.last_mut().unwrap() = placed;
                    break 'atlas_maker;
                }
            }

            // create new atlas if current is beyond max limit
            if *sizes.last().unwrap() >= max {
                if new_atlas {
                    *sizes.last_mut().unwrap() = og_size;
                }
                atlases.push(vec![]);
                sizes.push(0);

                // since this is a new atlas, keep only this style's textures
                boxes = style_boxes.clone();
            }
        }
    }

    let mut bufs = vec![];

    for i in 0..atlases.len() {
        let mut raw_buf =
            <image::ImageBuffer<image::Rgba<u8>, _>>::new(sizes[i] as u32, sizes[i] as u32);

        for s in 0..armature.styles.len() {
            for t in 0..armature.styles[s].textures.len() {
                let tex = &armature.styles[s].textures[t];
                if tex.atlas_idx != i as i32 {
                    continue;
                }

                let p = atlases[i]
                    .iter()
                    .position(|pl| pl.width == tex.size.x as i32 && pl.height == tex.size.y as i32);

                if p == None {
                    continue;
                }

                let offset_x = atlases[i][p.unwrap()].get_coords().0 as u32;
                let offset_y = atlases[i][p.unwrap()].get_coords().2 as u32;

                // ensure another tex of the same size won't overwrite this one
                atlases[i].remove(p.unwrap());

                raw_buf
                    .copy_from(&armature.tex_data(tex).unwrap().image, offset_x, offset_y)
                    .unwrap();

                armature.styles[s].textures[t].offset = Vec2::new(offset_x as f32, offset_y as f32);
            }
        }

        // encode buffer to png
        let mut png_buf: Vec<u8> = vec![];
        let encoder = image::codecs::png::PngEncoder::new(&mut png_buf);
        let img = image::ColorType::Rgba8.into();
        encoder
            .write_image(&raw_buf, raw_buf.width(), raw_buf.height(), img)
            .unwrap();

        bufs.push(png_buf);
    }

    (bufs, sizes)
}

pub fn prepare_files(
    armature: &Armature,
    camera: Camera,
    sizes: Vec<i32>,
    selection: &SelectionState,
    edit_mode: &EditMode,
) -> (String, String) {
    // clone armature and make some edits, then serialize it
    let mut armature_copy = armature.clone();

    for a in 0..armature_copy.animations.len() {
        if !edit_mode.export_bake_ik {
            break;
        }
        armature_copy.animations[a].id = a as i32;
        let mut extra_keyframes: Vec<Keyframe> = vec![];
        let mut last_frame_rots: HashMap<i32, f32> = HashMap::new();
        for kf in 0..armature_copy.animations[a].keyframes.len() {
            let keyframe = armature_copy.animations[a].keyframes[kf].clone();
            let bones = &armature_copy.bones;
            let bone = bones.iter().find(|b| b.id == keyframe.bone_id).unwrap();

            let mut bones = armature.bones.iter();
            let root_idx = bones.position(|b| b.ik_family_id != -1 && b.ik_target_id == bone.id);
            if root_idx == None {
                continue;
            }

            let mut anim_arm = armature_copy.clone();
            anim_arm.bones = anim_arm.animate(a, keyframe.frame, None);
            let bones = anim_arm.bones.clone();
            construction(&mut anim_arm.bones, &bones);

            let ifd = anim_arm.bones[root_idx.unwrap()].ik_family_id;
            let bones = &mut anim_arm.bones.iter_mut();
            let mut family: Vec<&mut Bone> = bones.filter(|b| b.ik_family_id == ifd).collect();
            for i in 0..family.len() {
                for f in 0..family.len() {
                    if family[f].id == family[i].id {
                        break;
                    }
                    family[i].rot -= family[f].rot;
                }
                if last_frame_rots.contains_key(&family[i].id) {
                    let prev_rot = *last_frame_rots.get(&family[i].id).unwrap();
                    family[i].rot = prev_rot + shortest_angle_delta(prev_rot, family[i].rot);
                    last_frame_rots.remove(&family[i].id);
                }
                last_frame_rots.insert(family[i].id, family[i].rot);

                let ae_rot = AnimElement::Rotation;
                let exists = extra_keyframes.iter().find(|kf| {
                    kf.frame == keyframe.frame && kf.bone_id == family[i].id && kf.element == ae_rot
                });
                if exists == None {
                    extra_keyframes.push(Keyframe {
                        frame: keyframe.frame,
                        bone_id: family[i].id,
                        element_id: AnimElement::Rotation as i32,
                        element: AnimElement::Rotation,
                        value_str: "".to_string(),
                        value: family[i].rot,
                        transition: keyframe.transition.clone(),
                        label_top: 0.,
                    });
                }
            }
        }

        armature_copy.animations[a]
            .keyframes
            .append(&mut extra_keyframes);
        armature_copy.animations[a].sort_keyframes();
    }

    if edit_mode.export_bake_ik && edit_mode.export_exclude_ik {
        for bone in &mut armature_copy.bones {
            bone.ik_family_id = -1;
        }
    } else {
        let mut family_ids: Vec<i32> = armature_copy
            .bones
            .iter()
            .map(|bone| bone.ik_family_id)
            .filter(|id| *id != -1)
            .collect();
        family_ids.dedup();
        for fid in family_ids {
            let ac_c = armature_copy.clone();
            let ac = &mut armature_copy;
            let mut joints: Vec<&mut Bone> = ac
                .bones
                .iter_mut()
                .filter(|b| b.ik_family_id == fid)
                .collect();

            // get all bone ids (sequentially) of this family in one array
            let mut bone_ids = vec![];
            for joint in &joints {
                let idx = ac_c.bones.iter().position(|bone| bone.id == joint.id);
                bone_ids.push(idx.unwrap() as i32);
            }

            // get target id (sequentially)
            let mut target_id = -1;
            let joint_target_id = joints[0].ik_target_id;
            let target_idx = ac_c.bones.iter().position(|b| b.id == joint_target_id);
            if target_idx != None {
                target_id = target_idx.unwrap() as i32;
            }

            // clear ik_bone_ids of al joints to mark them as 'non-root'
            for joint in &mut joints {
                joint.ik_bone_ids = vec![];
            }

            // populate IK data to root bone
            joints[0].ik_bone_ids = bone_ids;
            joints[0].ik_target_id = target_id;
            joints[0].ik_constraint_id = joints[0].ik_constraint as i32;
            joints[0].ik_mode_id = joints[0].ik_mode as i32;
        }
    }

    for a in 0..armature_copy.animations.len() {
        armature_copy.animations[a].id = a as i32;
        for kf in 0..armature_copy.animations[a].keyframes.len() {
            let keyframe = &mut armature_copy.animations[a].keyframes[kf];
            let bones = &mut armature_copy.bones.iter();

            // populate keyframe bone_id
            keyframe.bone_id = bones.position(|bone| bone.id == keyframe.bone_id).unwrap() as i32;

            // populate value_str of constraint keyframes
            if keyframe.element == AnimElement::IkConstraint {
                keyframe.value_str = match keyframe.value {
                    1. => "Clockwise".to_string(),
                    2. => "CounterClockwise".to_string(),
                    _ => "None".to_string(),
                };
            }
        }
    }

    for b in 0..armature_copy.bones.len() {
        // if it's a regular rect, empty verts and indices
        let bone = &armature_copy.bones[b];
        if armature_copy.tex_of(bone.id) == None || !bone.verts_edited {
            armature_copy.bones[b].vertices = vec![];
            armature_copy.bones[b].indices = vec![];
            continue;
        }

        for w in 0..armature_copy.bones[b].binds.len() {
            let bones = armature_copy.bones.clone();
            let bone_id = &mut armature_copy.bones[b].binds[w].bone_id;
            *bone_id = bones.iter().position(|bone| bone.id == *bone_id).unwrap() as i32;
            for v in 0..armature_copy.bones[b].binds[w].verts.len() {
                let vertices = armature_copy.bones[b].vertices.clone();
                let v_id = &mut armature_copy.bones[b].binds[w].verts[v].id;
                *v_id = vertices.iter().position(|v| v.id == *v_id as u32).unwrap() as i32;
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
        bone.init_tex = bone.tex.clone();
        bone.init_is_hidden = bone.is_hidden;
        bone.init_ik_constraint = bone.ik_constraint_id;

        if bone.ik_bone_ids.len() == 0 {
            bone.ik_constraint = JointConstraint::Skip;
            bone.ik_mode = InverseKinematicsMode::Skip;
            bone.ik_family_id = -1;
            bone.ik_bone_ids = vec![];
            bone.ik_mode_id = -1;
            bone.ik_constraint_id = -1;
            bone.init_ik_constraint = -1;
        }
    }

    for b in 0..armature_copy.bones.len() {
        if armature_copy.bones[b].parent_id == -1 {
            continue;
        }

        let bones = armature_copy.bones.clone();
        let parent_id = &mut armature_copy.bones[b].parent_id;
        *parent_id = bones.iter().position(|bone| bone.id == *parent_id).unwrap() as i32;
    }

    // restructure bone ids
    for b in 0..armature_copy.bones.len() {
        let bone = &mut armature_copy.bones[b];
        if bone.tex == "" {
            bone.zindex = -1;
        }

        bone.id = b as i32;
    }

    let mut ik_root_ids = vec![];
    for bone in &armature_copy.bones {
        if bone.ik_family_id != -1 {
            ik_root_ids.push(bone.id);
        }
    }

    // populate texture ser_offset and ser_size
    for s in 0..armature.styles.len() {
        armature_copy.styles[s].id = s as i32;
        for t in 0..armature.styles[s].textures.len() {
            let tex = &mut armature_copy.styles[s].textures[t];
            tex.ser_offset = Vec2I::new(tex.offset.x as i32, tex.offset.y as i32);
            tex.ser_size = Vec2I::new(tex.size.x as i32, tex.size.y as i32);
        }
    }

    let mut atlases = vec![];
    for s in 0..sizes.len() {
        atlases.push(TexAtlas {
            filename: "atlas".to_owned() + &s.to_string() + ".png",
            size: Vec2I::new(sizes[s], sizes[s]),
        });
    }

    let root = Root {
        version: env!("CARGO_PKG_VERSION").to_string(),
        ik_root_ids,
        baked_ik: edit_mode.export_bake_ik,
        bones: armature_copy.bones,
        animations: armature_copy.animations,
        styles: armature_copy.styles,
        atlases,
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
            effects_folded: bone.effects_folded,
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
        tex_data: vec![],
        animated_bones: vec![],
    };

    for bone in &mut shared.armature.bones {
        for (i, vert) in bone.vertices.iter_mut().enumerate() {
            vert.id = i as u32;
        }

        bone.verts_edited = bone.vertices.len() > 0;
    }

    // populate style ids
    for s in 0..shared.armature.styles.len() {
        shared.armature.styles[s].id = s as i32;
    }

    // populate bone IK data
    for b in 0..shared.armature.bones.len() {
        if shared.armature.bones[b].ik_bone_ids.len() > 0 {
            for i in 0..shared.armature.bones[b].ik_bone_ids.len() {
                let id = shared.armature.bones[b].ik_bone_ids[i] as i32;
                let fam_id = shared.armature.bones[b].ik_family_id;
                let bones = &mut shared.armature.bones;
                bones.iter_mut().find(|b| b.id == id).unwrap().ik_family_id = fam_id;
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
        let mut imgs = vec![];
        for a in 0..root.atlases.len() {
            let name = &("atlas".to_owned() + &a.to_string() + ".png");
            let texture_file = zip.as_mut().unwrap().by_name(name).unwrap();

            let mut bytes = vec![];
            for byte in texture_file.bytes() {
                bytes.push(byte.unwrap());
            }
            imgs.push(image::load_from_memory(&bytes).unwrap());
        }

        for set in &mut shared.armature.styles {
            for tex in &mut set.textures {
                tex.offset = Vec2::new(tex.ser_offset.x as f32, tex.ser_offset.y as f32);
                tex.size = Vec2::new(tex.ser_size.x as f32, tex.ser_size.y as f32);
                let u_offset_x = tex.offset.x as u32;
                let u_offset_y = tex.offset.y as u32;
                let u_size_x = tex.size.x as u32;
                let u_size_y = tex.size.y as u32;

                let image =
                    imgs[tex.atlas_idx as usize].crop(u_offset_x, u_offset_y, u_size_x, u_size_y);
                let mut bind_group: Option<wgpu::BindGroup> = None;

                if queue != None && device != None && bind_group_layout != None {
                    bind_group = Some(renderer::create_texture_bind_group(
                        image.clone().into_rgba8().to_vec(),
                        tex.size,
                        queue.unwrap(),
                        device.unwrap(),
                        bind_group_layout.unwrap(),
                    ));
                }

                if context == None {
                    continue;
                }

                let img =
                    imgs[tex.atlas_idx as usize].crop(u_offset_x, u_offset_y, u_size_x, u_size_y);
                let filter = image::imageops::FilterType::Nearest;
                let pixels = img.resize_exact(300, 300, filter).into_rgba8().to_vec();

                let col = egui::ColorImage::from_rgba_unmultiplied([300, 300], &pixels);
                let file = "anim_icons";
                let ui_img = context.unwrap().load_texture(file, col, Default::default());

                let data_id = shared.armature.tex_data.len() as i32;
                tex.data_id = data_id;
                shared.armature.tex_data.push(TextureData {
                    id: data_id,
                    image,
                    bind_group,
                    ui_img: Some(ui_img),
                });
            }
        }
    }

    shared.events.unselect_all();
    shared.ui.startup_window = false;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_to_recent_files(paths: &Vec<String>) {
    fs::create_dir_all(recents_path().parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(&recents_path()).unwrap();
    file.write_all(serde_json::to_string(&paths).unwrap().as_bytes())
        .unwrap();
}

// unused: use `bone.verts_edited` instead
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
        let config_json = serde_json::to_string(&config).unwrap();
        let mut config_file = std::fs::File::create(&config_path()).unwrap();
        config_file.write_all(config_json.as_bytes()).unwrap();

        fs::create_dir_all(color_path().parent().unwrap()).unwrap();
        let color_json = serde_json::to_string(&config.colors).unwrap();
        let mut color_file = std::fs::File::create(&color_path()).unwrap();
        color_file.write_all(color_json.as_bytes()).unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    {
        saveConfig(serde_json::to_string(config).unwrap());
        updateUiSlider();
    }
}

pub fn config_str() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut str = String::new();
        std::fs::File::open(&config_path())
            .unwrap()
            .read_to_string(&mut str)
            .unwrap();
        return str;
    }
    #[cfg(target_arch = "wasm32")]
    {
        return getConfig();
    }
}

pub fn color_str() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut str = String::new();
        std::fs::File::open(&color_path())
            .unwrap()
            .read_to_string(&mut str)
            .unwrap();
        return str;
    }
    #[cfg(target_arch = "wasm32")]
    {
        return getConfig();
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
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();
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

// Simulate text being added to egui and truncate it to fit the max width
pub fn trunc_str(ui: &egui::Ui, text: &str, max_width: f32) -> String {
    let f_id = egui::FontId::proportional(14.0);
    let col = egui::Color32::WHITE;
    let mut width = ui.ctx().fonts_mut(|fonts| {
        let galley = fonts.layout_no_wrap(text.to_string(), f_id.clone(), col);
        galley.size().x
    });
    let mut ctext = text.to_string();
    let elipsis_margin = 7.;
    while width + elipsis_margin > max_width {
        width = ui.ctx().fonts_mut(|fonts| {
            ctext.pop();
            let galley = fonts.layout_no_wrap(ctext.to_string(), f_id.clone(), col);
            galley.size().x
        });
    }
    if ctext.len() < text.len() {
        ctext += "...";
    }
    ctext
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

pub fn exit(undo_states: &mut UndoStates, config: &Config, ui: &mut Ui) {
    if undo_states.undo_actions.len() == 0 && !config.ignore_donate {
        ui.donating_modal = true;
    } else if !ui.donating_modal {
        ui.exiting = true;
    }
}

pub fn animate_bones(armature: &mut Armature, selection: &SelectionState, edit_mode: &EditMode) {
    // runtime:
    // armature bones should normally be mutable to animation for smoothing,
    // but that's not ideal when editing
    armature.animated_bones = armature.bones.clone();

    let anims = &armature.animations;
    let is_any_anim_playing = anims.iter().find(|anim| anim.elapsed != None) != None;

    if is_any_anim_playing {
        // runtime: playing animations (single & simultaneous)
        for a in 0..armature.animations.len() {
            let anim = &mut armature.animations[a];
            if anim.elapsed == None {
                continue;
            }
            let frame = anim.set_frame();
            let anim_bones = armature.animated_bones.clone();
            armature.animated_bones = armature.animate(a, frame, Some(&anim_bones));
        }
    } else if edit_mode.anim_open && selection.anim != usize::MAX && selection.anim_frame != -1 {
        // display the selected animation's frame
        armature.animated_bones = armature.animate(selection.anim, selection.anim_frame, None);
    }
}

pub fn crashlog_file() -> PathBuf {
    let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    let exe_dir = exe_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    exe_dir.join("crash.log")
}

pub fn interp(current: i32, max: i32, start_val: f32, end_val: f32, transition: Transition) -> f32 {
    if max == 0 || current >= max {
        return end_val;
    }
    let interp = match transition {
        Transition::Linear => current as f32 / max as f32,
        Transition::SineIn => 1. - (current as f32 / max as f32 * 3.14 * 0.5).cos(),
        Transition::SineOut => (current as f32 / max as f32 * 3.14 * 0.5).sin(),
    };
    start_val + (end_val - start_val) * interp
}

// I admit defeat:
// https://chatgpt.com/share/697de90a-5a08-8004-9551-326e2ba6aee2
pub fn shortest_angle_delta(from: f32, to: f32) -> f32 {
    let mut delta = to - from;
    delta = (delta + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    delta
}
