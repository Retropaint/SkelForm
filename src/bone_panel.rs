//! UI Bone window.

use crate::*;

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
        ui.label("Name:");
        let (edited, value, _) = ui::text_input(
            "Name".to_string(),
            shared,
            ui,
            shared.selected_bone().unwrap().name.clone(),
            None,
        );
        if edited {
            shared.selected_bone_mut().unwrap().name = value;
        }
        //ui.text_edit_singleline(&mut shared.selected_bone_mut().unwrap().name)
        //    .labelled_by(l.id);
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
            (edited, $float, _) = ui::float_input($id.to_string(), shared, $ui, $float, $modifier);
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
            (edited, $float, $input) =
                ui::float_input($id.to_string(), shared, $ui, $float, $modifier);
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
            ui::draw_tutorial_rect(TutorialStep::EditBoneY, input.rect, shared, ui);
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
            ui::draw_tutorial_rect(TutorialStep::EditBoneX, input.rect, shared, ui);
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

    if shared.selected_bone().unwrap().vertices.len() == 0 {
        return;
    }

    let mut mesh_label = "Edit Mesh";
    if shared.editing_mesh {
        mesh_label = "Finish Edit";
    }

    ui.horizontal(|ui| {
        if ui::button(mesh_label, ui).clicked() {
            shared.editing_mesh = !shared.editing_mesh;
        }

        if shared.editing_mesh {
            if ui::button("Reset", ui).clicked() {
                let tex = &shared.armature.textures[bone.tex_idx as usize];
                (
                    shared.selected_bone_mut().unwrap().vertices,
                    shared.selected_bone_mut().unwrap().indices,
                ) = renderer::create_tex_rect(tex);
            }
        }
    });

    if !shared.editing_mesh {
        return;
    }

    ui.horizontal(|ui| {
        ui.label("Base Index:");
        let base = shared.selected_bone().unwrap().indices[0] as f32;
        let (edited, base, _) = ui::float_input("base_index".to_string(), shared, ui, base, 1.);
        if edited {
            shared.selected_bone_mut().unwrap().indices = crate::renderer::setup_indices(
                &shared.selected_bone_mut().unwrap().vertices,
                base as i32,
            );
        }
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
