//! UI Bone window.

use crate::{
    shared::*,
    ui::{self, draw_tutorial_rect},
};

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
    pub fn toggleElement(open: bool, id: String);
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
                    .size(12.)
                    .color(egui::Color32::DARK_RED),
            ),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        shared.ui.open_polar_modal(
            PolarId::DeleteBone,
            "Are you sure to delete this bone?".to_string(),
        );
    }

    ui.separator();
    ui.add_space(3.);

    if shared.ui.has_state(UiState::DraggingBone) {
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
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = ui::button("Get Image", ui);
            ui::draw_tutorial_rect(TutorialStep::GetImage, button.rect, shared, ui);
            if button.clicked() {
                if shared.bind_groups.len() == 0 {
                    #[cfg(not(target_arch = "wasm32"))]
                    open_file_dialog();

                    #[cfg(target_arch = "wasm32")]
                    toggleElement(true, "image-dialog".to_string());
                } else {
                    shared.ui.set_state(UiState::ImageModal, true);
                }
                shared.start_next_tutorial_step(TutorialStep::EditBoneX);
            };
            let mut tex_name = "None";
            if shared.selected_bone().unwrap().tex_idx != -1 {
                tex_name =
                    &shared.armature.textures[shared.selected_bone().unwrap().tex_idx as usize].name
            }
            ui.label(tex_name);
        })
    });

    ui.add_space(3.5);

    let mut bone = shared.selected_bone().unwrap().clone();
    if shared.ui.anim.open && shared.ui.anim.selected != usize::MAX {
        bone = shared.animate(shared.ui.anim.selected)[shared.selected_bone_idx].clone();
    }

    let mut edited = false;

    macro_rules! input {
        ($float:expr, $id:expr, $element:expr, $modifier:expr, $ui:expr, $label:expr) => {
            (edited, $float, _) = float_input($id.to_string(), shared, $ui, $float, $modifier);
            if edited {
                shared.save_edited_bone();
                shared.edit_bone($element, $float, true);
            }
            if $label != "" {
                $ui.label($label);
            }
        };
    }

    macro_rules! input_response {
        ($float:expr, $id:expr, $element:expr, $modifier:expr, $ui:expr, $label:expr, $input:expr) => {
            (edited, $float, $input) = float_input($id.to_string(), shared, $ui, $float, $modifier);
            if edited {
                shared.save_edited_bone();
                shared.edit_bone($element, $float, true);
            }
            if $label != "" {
                $ui.label($label);
            }
        };
    }

    macro_rules! label {
        ($name:expr, $ui:expr) => {
            $ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label($name);
            });
        };
    }

    ui.horizontal(|ui| {
        label!("Position:", ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let input: egui::Response;
            input_response!(
                bone.pos.y,
                "pos_y",
                &AnimElement::PositionY,
                1.,
                ui,
                "Y",
                input
            );
            draw_tutorial_rect(TutorialStep::EditBoneY, input.rect, shared, ui);
            if edited {
                shared.start_next_tutorial_step(TutorialStep::OpenAnim);
            }

            let input: egui::Response;
            input_response!(
                bone.pos.x,
                "pos_x",
                &AnimElement::PositionX,
                1.,
                ui,
                "X",
                input
            );
            draw_tutorial_rect(TutorialStep::EditBoneX, input.rect, shared, ui);
            if edited {
                shared.start_next_tutorial_step(TutorialStep::EditBoneY);
            }
        })
    });
    ui.horizontal(|ui| {
        label!("Scale:", ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            input!(bone.scale.y, "scale_y", &AnimElement::ScaleY, 1., ui, "Y");
            input!(bone.scale.x, "scale_x", &AnimElement::ScaleX, 1., ui, "X");
        });
    });
    ui.horizontal(|ui| {
        label!("Pivot:", ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            input!(bone.pivot.y, "pivot_y", &AnimElement::PivotY, 1., ui, "Y");
            input!(bone.pivot.x, "pivot_x", &AnimElement::PivotX, 1., ui, "X");
        });
    });
    ui.horizontal(|ui| {
        label!("Rotation:", ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            input!(
                bone.rot,
                "rot",
                &AnimElement::Rotation,
                180. / std::f32::consts::PI,
                ui,
                ""
            );
        });
    });
    ui.horizontal(|ui| {
        label!("Z-Index:", ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            input!(bone.zindex, "zindex", &AnimElement::Zindex, 1., ui, "");
        });
    });
    if shared.selected_bone().unwrap().vertices.len() > 0 {
        let mut mesh_label = "Edit Mesh";
        if shared.editing_mesh {
            mesh_label = "Finish Edit";
        }

        if ui::button(mesh_label, ui).clicked() {
            shared.editing_mesh = !shared.editing_mesh;
        }
    }
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
    modifier: f32,
) -> (bool, f32, egui::Response) {
    let displayed_value = value * modifier;

    let input: egui::Response;

    let input_size = [40., 20.];

    if shared.ui.rename_id != id {
        input = ui.add_sized(
            input_size,
            egui::TextEdit::singleline(&mut displayed_value.to_string())
                .desired_width(0.)
                .min_size(egui::Vec2::ZERO),
        );
        // extract value as a string and store it with edit_value
        if input.has_focus() {
            shared.ui.edit_value = Some(displayed_value.to_string());
            shared.ui.rename_id = id.to_string();
        }
    } else {
        input = ui.add_sized(
            input_size,
            egui::TextEdit::singleline(shared.ui.edit_value.as_mut().unwrap())
        );

        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            shared.ui.rename_id = "".to_string();
            if shared.ui.edit_value.as_mut().unwrap() == "" {
                shared.ui.edit_value = Some("0".to_string());
            }
            match shared.ui.edit_value.as_mut().unwrap().parse::<f32>() {
                Ok(output) => {
                    return (true, output / modifier, input);
                }
                Err(_) => {
                    return (false, value, input);
                }
            }
        }

        if input.lost_focus() {
            shared.ui.rename_id = "".to_string();
        }
    }
    (false, value, input)
}
