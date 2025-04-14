//! UI Bone window.

use egui::*;

use crate::{shared::*, ui as ui_mod};
use std::f32::consts::PI;

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use std::{fs::File, io::Write, thread};
}
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

// web-only imports
#[cfg(target_arch = "wasm32")]
mod web {
    pub use crate::wasm_bindgen::*;
    pub use wasm_bindgen::prelude::wasm_bindgen;
    pub use web_sys::js_sys::wasm_bindgen;
}
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn toggleFileDialog(open: bool);
}

pub fn draw(egui_ctx: &Context, shared: &mut Shared) {
    egui::SidePanel::right("Bone")
        .resizable(true)
        .default_width(150.)
        .max_width(200.)
        .show(egui_ctx, |ui| {
            ui.heading("Bone");
            ui.separator();
            ui.add_space(3.);

            shared.ui.animate_mode_bar_pos.x = ui.min_rect().left();

            if shared.selected_bone_idx == usize::MAX || shared.dragging {
                ui.disable();
                return;
            }

            // shorthand
            if ui_mod::button("Delete Bone", ui).clicked() {
                shared.armature.bones.remove(shared.selected_bone_idx);
                shared.selected_bone_idx = usize::MAX;
                return;
            };

            ui.horizontal(|ui| {
                let l = ui.label("Name:");
                ui.text_edit_singleline(&mut shared.selected_bone_mut().name).labelled_by(l.id);
            });
            ui.horizontal(|ui| {
                ui.label("Texture:");
                if ui_mod::button("Get Image", ui).clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let bone_idx = shared.selected_bone_idx;
                        open_file_dialog(bone_idx);
                    }

                    #[cfg(target_arch = "wasm32")]
                    toggleFileDialog(true);
                };
            });
            if shared.selected_bone_idx == usize::MAX {
                return;
            }
            ui.label("Position:");
            ui.horizontal(|ui| {
                ui.label("X");
                float_input(ui, &mut shared.selected_bone_mut().pos.x);
                ui.label("Y");
                float_input(ui, &mut shared.selected_bone_mut().pos.y);
            });
            ui.label("Scale:");
            ui.horizontal(|ui| {
                ui.label("X");
                float_input(ui, &mut shared.selected_bone_mut().scale.x);
                ui.label("Y");
                float_input(ui, &mut shared.selected_bone_mut().scale.y);
            });
            ui.horizontal(|ui| {
                ui.label("Rotation");
                let deg = shared.selected_bone().rot / PI * 180.;
                let mut str = deg.round().to_string();
                if !str.contains(".") {
                    str.push('.');
                }
                ui.add_sized([30., 20.], egui::TextEdit::singleline(&mut str));
                if let Ok(f) = str.parse::<f32>() {
                    shared.selected_bone_mut().rot = f * PI / 180.;
                } else {
                    shared.selected_bone_mut().rot = 0.;
                }
            });
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn open_file_dialog(bone_idx: usize) {
    #[cfg(not(target_arch = "wasm32"))]
    thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg"])
            .pick_file();
        if task == None {
            return
        }
        let mut img_path = File::create(".skelform_img_path").unwrap();
        img_path
            .write_all(task.unwrap().as_path().to_str().unwrap().as_bytes())
            .unwrap();
    });
}

// helper for editable float inputs
fn float_input(ui: &mut egui::Ui, float: &mut f32) {
    let truncated = (*float * 100.).trunc() / 100.;
    let mut str = truncated.to_string();
    if !str.contains(".") {
        str.push('.');
    }
    ui.add_sized([40., 20.], egui::TextEdit::singleline(&mut str));
    if let Ok(f) = str.parse::<f32>() {
        *float = f;
    } else {
        *float = 0.;
    }
}
