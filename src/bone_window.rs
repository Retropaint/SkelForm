//! UI Bone window.

use egui::*;
use web_sys::js_sys::wasm_bindgen;

use crate::shared::*;
use std::f32::consts::PI;
use wasm_bindgen::prelude::*;

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use std::{fs::File, io::Write, thread};
}
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[wasm_bindgen]
extern "C" {
    fn toggleFileDialog(open: bool);
}

pub fn draw(egui_ctx: &Context, shared: &mut Shared) {
    egui::Window::new("Bone")
        .movable(false)
        .anchor(Align2::RIGHT_TOP, Vec2 { x: 0., y: 0. })
        .max_width(150.)
        .collapsible(false)
        .resizable(false)
        .show(egui_ctx, |ui| {
            let mut bone: Bone = Bone::default();
            if shared.selected_bone != usize::MAX {
                bone = shared.armature.bones[shared.selected_bone].clone();
            } else {
                ui.disable();
            }
            ui.horizontal(|ui| {
                let l = ui.label("Name:");
                ui.text_edit_singleline(&mut bone.name).labelled_by(l.id);
            });
            ui.horizontal(|ui| {
                ui.label("Texture:");
                if ui.button("Get Image").clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let bone_idx = shared.selected_bone;
                        open_file_dialog(bone_idx);
                    }

                    #[cfg(target_arch = "wasm32")]
                    toggleFileDialog(true);
                };
            });
            if shared.selected_bone == usize::MAX {
                return;
            }
            ui.horizontal(|ui| {
                ui.label("Position:");
                ui.label("x:");
                float_input(ui, &mut shared.armature.bones[shared.selected_bone].pos.x);
                ui.label("y:");
                float_input(ui, &mut shared.armature.bones[shared.selected_bone].pos.y);
            });
            ui.horizontal(|ui| {
                ui.label("Scale:");
                ui.label("x:");
                float_input(ui, &mut shared.armature.bones[shared.selected_bone].scale.x);
                ui.label("y:");
                float_input(ui, &mut shared.armature.bones[shared.selected_bone].scale.y);
            });
            ui.horizontal(|ui| {
                ui.label("Rotation:");
                let deg = shared.armature.bones[shared.selected_bone].rot / PI * 180.;
                let mut str = deg.round().to_string();
                if !str.contains(".") {
                    str.push('.');
                }
                ui.add_sized([30., 20.], egui::TextEdit::singleline(&mut str));
                if let Ok(f) = str.parse::<f32>() {
                    shared.armature.bones[shared.selected_bone].rot = f * PI / 180.;
                } else {
                    shared.armature.bones[shared.selected_bone].rot = 0.;
                }
            });
            if ui.button("Delete Bone").clicked() {
                shared.armature.bones.remove(shared.selected_bone);
                shared.selected_bone = usize::MAX;
            };
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn open_file_dialog(bone_idx: usize) {
    #[cfg(not(target_arch = "wasm32"))]
    thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg"])
            .pick_file();
        let mut img_path = File::create(".skelform_img_path").unwrap();
        img_path
            .write_all(task.unwrap().as_path().to_str().unwrap().as_bytes())
            .unwrap();
        let mut bone_idx_file = File::create(".skelform_bone_idx").unwrap();
        bone_idx_file
            .write_all(bone_idx.to_string().as_bytes())
            .unwrap();
    });
}

// helper for editable float inputs
fn float_input(ui: &mut Ui, float: &mut f32) {
    let mut str = float.to_string();
    if !str.contains(".") {
        str.push('.');
    }
    ui.add_sized([30., 20.], egui::TextEdit::singleline(&mut str));
    if let Ok(f) = str.parse::<f32>() {
        *float = f;
    } else {
        *float = 0.;
    }
}
