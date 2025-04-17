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

            ui.horizontal(|ui| {
                let l = ui.label("Name:");
                ui.text_edit_singleline(&mut shared.selected_bone_mut().name)
                    .labelled_by(l.id);
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

            let mut bone = shared.selected_bone().clone();
            if shared.animating && shared.ui.anim.selected != usize::MAX {
                bone = shared.animate(shared.ui.anim.selected, shared.ui.anim.selected_frame)
                    [shared.selected_bone_idx]
                    .clone();
            }
            let mut edited = false;
            ui.label("Position:");
            ui.horizontal(|ui| {
                ui.label("X");
                (edited, bone.pos.x) =
                    float_input("pos_x".to_string(), shared, ui, bone.pos.x, None);
                if edited {
                    shared.edit_bone(0, bone.pos);
                }
                ui.label("Y");
                (edited, bone.pos.y) =
                    float_input("pos_y".to_string(), shared, ui, bone.pos.y, None);
                if edited {
                    shared.edit_bone(0, bone.pos);
                }
            });
            ui.label("Scale:");
            ui.horizontal(|ui| {
                ui.label("X");
                (edited, bone.scale.x) =
                    float_input("scale_x".to_string(), shared, ui, bone.scale.x, None);
                if edited {
                    shared.edit_bone(2, bone.scale);
                }
                ui.label("Y");
                (edited, bone.scale.y) =
                    float_input("scale_y".to_string(), shared, ui, bone.scale.y, None);
                if edited {
                    shared.edit_bone(2, bone.scale);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Rotation:");
                (edited, bone.rot) = float_input(
                    "rot".to_string(),
                    shared,
                    ui,
                    bone.rot,
                    Some(180. / std::f32::consts::PI),
                );
                if edited {
                    shared.edit_bone(1, crate::shared::Vec2::single(bone.rot));
                }
            });

            if ui_mod::button("Delete Bone", ui).clicked() {
                //shared.armature.bones.remove(shared.selected_bone_idx);
                //shared.selected_bone_idx = usize::MAX;
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
        if task == None {
            return;
        }
        let mut img_path = File::create(".skelform_img_path").unwrap();
        img_path
            .write_all(task.unwrap().as_path().to_str().unwrap().as_bytes())
            .unwrap();
    });
}

// helper for editable float inputs
fn float_input(
    id: String,
    shared: &mut Shared,
    ui: &mut egui::Ui,
    value: f32,
    mut modifier: Option<f32>,
) -> (bool, f32) {
    let displayed_value;
    if modifier != None {
        displayed_value = value * modifier.unwrap();
    } else {
        displayed_value = value * 1.;
        modifier = Some(1.);
    }
    if shared.ui.rename_id != id {
        let input = ui.add_sized(
            [40., 20.],
            egui::TextEdit::singleline(&mut displayed_value.to_string()),
        );
        if input.has_focus() {
            shared.ui.edit_value = Some(displayed_value.to_string());
            shared.ui.rename_id = id.to_string();
        }
    } else {
        let input = ui.add_sized(
            [40., 20.],
            egui::TextEdit::singleline(shared.ui.edit_value.as_mut().unwrap()),
        );
        if input.lost_focus() {
            shared.ui.rename_id = "".to_string();
            if shared.ui.edit_value.as_mut().unwrap() == "" {
                shared.ui.edit_value = Some("0".to_string());
            }
            return (
                true,
                shared
                    .ui
                    .edit_value
                    .as_mut()
                    .unwrap()
                    .parse::<f32>()
                    .unwrap()
                    / modifier.unwrap(),
            );
        }
    }
    (false, value)
}
