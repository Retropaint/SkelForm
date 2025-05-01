//! UI Bone window.

use egui::*;

use crate::{shared::*, ui as ui_mod};

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use crate::file_reader::*;
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
    pub fn toggleFileDialog(open: bool);
}

pub fn draw(ui: &mut egui::Ui, shared: &mut Shared) {
    ui.heading("Bone");

    // delete label
    let delete_rect = egui::Rect::from_min_size(ui.min_rect().right_top(), egui::Vec2::ZERO);
    if ui
        .put(
            delete_rect,
            egui::Label::new(
                egui::RichText::new("X")
                    .size(18.)
                    .color(egui::Color32::DARK_RED),
            ),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        shared.ui.polar_id = "delete_bone".to_string();
        shared.ui.polar_headline = "Are you sure to delete this bone?".to_string();
    }

    ui.separator();
    ui.add_space(3.);

    if shared.dragging {
        ui.disable();
        return;
    }

    ui.horizontal(|ui| {
        let l = ui.label("Name:");
        ui.text_edit_singleline(&mut shared.selected_bone_mut().unwrap().name)
            .labelled_by(l.id);
    });

    ui.horizontal(|ui| {
        ui.label("Texture:");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if ui_mod::button("Get Image", ui).clicked() {
                if shared.bind_groups.len() == 0 {
                    #[cfg(not(target_arch = "wasm32"))]
                    open_file_dialog();

                    #[cfg(target_arch = "wasm32")]
                    toggleFileDialog(true);
                } else {
                    shared.ui.image_modal = true;
                }
            };
        })
    });

    ui.add_space(3.5);

    let mut bone = shared.selected_bone().unwrap().clone();
    if shared.animating && shared.ui.anim.selected != usize::MAX {
        bone = shared.animate(shared.ui.anim.selected)[shared.selected_bone_idx].clone();
    }

    let mut edited = false;

    macro_rules! input {
        ($element:expr, $float:expr, $id:expr, $edit_id:expr, $modifier:expr, $ui:expr, $label:expr) => {
            (edited, $float) = float_input($id.to_string(), shared, $ui, $float, $modifier);
            if edited {
                shared.save_edited_bone();
                shared.edit_bone($edit_id, $element, true);
            }
            if $label != "" {
                $ui.label($label);
            }
        };
    }

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.label("Position:");
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            input!(bone.pos, bone.pos.y, "pos_y", 0, None, ui, "Y");
            input!(bone.pos, bone.pos.x, "pos_x", 0, None, ui, "X");
        })
    });
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.label("Scale:");
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            input!(bone.scale, bone.scale.y, "scale_y", 2, None, ui, "Y");
            input!(bone.scale, bone.scale.x, "scale_x", 2, None, ui, "X");
        });
    });
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.label("Pivot:");
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            input!(bone.pivot, bone.pivot.y, "pivot_y", 3, None, ui, "Y");
            input!(bone.pivot, bone.pivot.x, "pivot_x", 3, None, ui, "X");
        });
    });
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.label("Rotation:");
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            input!(
                crate::Vec2::single(bone.rot),
                bone.rot,
                "rot",
                1,
                Some(180. / std::f32::consts::PI),
                ui,
                ""
            );
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_file_dialog() {
    #[cfg(not(target_arch = "wasm32"))]
    thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg"])
            .pick_file();
        if task == None {
            return;
        }
        create_temp_file(TEMP_IMG_PATH, task.unwrap().as_path().to_str().unwrap());
    });
}

// helper for editable float inputs
pub fn float_input(
    id: String,
    shared: &mut Shared,
    ui: &mut egui::Ui,
    value: f32,
    modifier: Option<f32>,
) -> (bool, f32) {
    let displayed_value;
    if modifier != None {
        displayed_value = value * modifier.unwrap();
    } else {
        displayed_value = value * 1.;
    }

    if shared.ui.rename_id != id {
        let input = ui.add_sized(
            [40., 20.],
            egui::TextEdit::singleline(&mut displayed_value.to_string()),
        );

        // extract value as a string and store it with edit_value
        if input.has_focus() {
            shared.ui.edit_value = Some(displayed_value.to_string());
            shared.ui.rename_id = id.to_string();
        }
    } else {
        let input = ui.add_sized(
            [40., 20.],
            egui::TextEdit::singleline(shared.ui.edit_value.as_mut().unwrap()),
        );

        // when done, parse and return edit_value
        if input.lost_focus() {
            shared.ui.rename_id = "".to_string();
            if shared.ui.edit_value.as_mut().unwrap() == "" {
                shared.ui.edit_value = Some("0".to_string());
            }
            match shared.ui.edit_value.as_mut().unwrap().parse::<f32>() {
                Ok(output) => {
                    return (true, output);
                }
                Err(_) => {
                    return (false, value);
                }
            }
        }
    }
    (false, value)
}
