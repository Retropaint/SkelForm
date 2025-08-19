//! UI Bone window.

use crate::*;
use ui::EguiUi;

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
    if shared.ui.has_state(UiState::DraggingBone) {
        ui.disable();
        return;
    }

    let mut bone = shared.selected_bone().unwrap().clone();
    if shared.ui.anim.open && shared.ui.anim.selected != usize::MAX {
        bone = shared
            .armature
            .animate(shared.ui.anim.selected, shared.ui.anim.selected_frame)
            [shared.ui.selected_bone_idx]
            .clone();
    }

    ui.horizontal(|ui| {
        ui.heading("Bone");

        // delete label
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let text = egui::RichText::new("X")
                .size(15.)
                .color(egui::Color32::DARK_RED);
            if ui
                .label(text)
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                shared.ui.context_menu.id = shared.selected_bone().unwrap().id;
                shared
                    .ui
                    .open_polar_modal(PolarId::DeleteBone, "Are you sure to delete this bone?");
            }
        });
    });

    ui.separator();
    ui.add_space(3.);

    ui.horizontal(|ui| {
        ui.label("Name:");
        let (edited, value, _) = ui.text_input(
            "Name".to_string(),
            shared,
            shared.selected_bone().unwrap().name.clone(),
            None,
        );
        if edited {
            shared.selected_bone_mut().unwrap().name = value;
        }
    });

    let set_name = if bone.tex_set_idx == -1 {
        "None".to_string()
    } else {
        shared.armature.texture_sets[bone.tex_set_idx as usize]
            .name
            .to_string()
    };

    let mut selected_set = bone.tex_set_idx;
    ui.horizontal(|ui| {
        ui.label("Tex. Set:");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::new("mod", "")
                .selected_text(set_name.to_string())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected_set, -1, "None");
                    let sets = &shared.armature.texture_sets;
                    for s in 0..sets.len() {
                        if sets[s].textures.len() == 0 {
                            continue;
                        }
                        ui.selectable_value(&mut selected_set, s as i32, sets[s].name.clone());
                    }
                    ui.selectable_value(&mut selected_set, -2, "[Setup]");
                })
                .response;
        });
    });
    if selected_set == -2 {
        shared.ui.selected_tex_set_idx = bone.tex_set_idx;
        shared.ui.set_state(UiState::ImageModal, true);
    } else if selected_set != bone.tex_set_idx {
        let mut anim_id = shared.ui.anim.selected;
        if !shared.ui.is_animating() {
            anim_id = usize::MAX;
        }
        shared.armature.set_bone_tex(
            bone.id,
            bone.tex_idx as usize,
            selected_set,
            anim_id,
            shared.ui.anim.selected_frame,
        );
    }

    if bone.tex_set_idx != -1 {
        let mut selected_tex = bone.tex_set_idx;
        let tex_name = &shared.armature.texture_sets[bone.tex_set_idx as usize].textures
            [bone.tex_idx as usize]
            .name;
        ui.horizontal(|ui| {
            ui.label("Tex. Index:");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::new("tex_selector", "")
                    .selected_text(tex_name.to_string())
                    .show_ui(ui, |ui| {
                        let set = &shared.armature.texture_sets[bone.tex_set_idx as usize];
                        for t in 0..set.textures.len() {
                            if set.textures.len() == 0 {
                                continue;
                            }
                            ui.selectable_value(
                                &mut selected_tex,
                                t as i32,
                                set.textures[t].name.clone(),
                            );
                        }
                        ui.selectable_value(&mut selected_tex, -2, "[Setup]");
                    })
                    .response;
            });
        });

        if selected_tex == -2 {
            shared.ui.selected_tex_set_idx = bone.tex_set_idx;
            shared.ui.set_state(UiState::ImageModal, true);
        } else if selected_tex != bone.tex_set_idx {
            let mut anim_id = shared.ui.anim.selected;
            if !shared.ui.is_animating() {
                anim_id = usize::MAX;
            }
            shared.armature.set_bone_tex(
                bone.id,
                selected_tex as usize,
                selected_set,
                anim_id,
                shared.ui.anim.selected_frame,
            );
        }
    }

    let mut edited = false;

    // Backbone of editable bone fields; do not use by itself. Instead refer to input!.
    macro_rules! check_input_edit {
        ($float:expr, $element:expr, $ui:expr, $label:expr) => {
            if edited {
                let mut anim_id = shared.ui.anim.selected;
                if !shared.ui.is_animating() {
                    anim_id = usize::MAX;
                }

                shared.save_edited_bone();
                shared.armature.edit_bone(
                    bone.id,
                    $element,
                    $float,
                    anim_id,
                    shared.ui.anim.selected_frame,
                );
            }
            if $label != "" {
                $ui.label($label);
            }
        };
    }

    // main macro to use for editable bone fields
    macro_rules! input {
        ($float:expr, $id:expr, $element:expr, $modifier:expr, $ui:expr, $label:expr) => {
            (edited, $float, _) = $ui.float_input($id.to_string(), shared, $float, $modifier);
            check_input_edit!($float, $element, $ui, $label)
        };
    }

    // same as input!, but provides back input response
    macro_rules! input_response {
        ($float:expr, $id:expr, $element:expr, $modifier:expr, $ui:expr, $label:expr, $input:expr) => {
            (edited, $float, $input) = $ui.float_input($id.to_string(), shared, $float, $modifier);
            check_input_edit!($float, $element, $ui, $label)
        };
    }

    // for labels that are not part of any input fields (eg "Position:", "Rotation:", etc)
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
            let pos_y = &AnimElement::PositionY;
            input_response!(bone.pos.y, "pos_y", pos_y, 1., ui, "Y", input);
            ui::draw_tutorial_rect(TutorialStep::EditBoneY, input.rect, shared, ui);
            if edited {
                shared
                    .ui
                    .start_next_tutorial_step(TutorialStep::OpenAnim, &shared.armature);
            }

            let input: egui::Response;
            let pos_x = &AnimElement::PositionX;
            input_response!(bone.pos.x, "pos_x", pos_x, 1., ui, "X", input);
            ui::draw_tutorial_rect(TutorialStep::EditBoneX, input.rect, shared, ui);
            if edited {
                shared
                    .ui
                    .start_next_tutorial_step(TutorialStep::EditBoneY, &shared.armature);
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
    // disabled: pivots are mostly superfluous as parent inheritance is mandatory
    //ui.horizontal(|ui| {
    //    label!("Pivot:", ui);
    //    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
    //        input!(bone.pivot.y, "pivot_y", &AnimElement::PivotY, 1., ui, "Y");
    //        input!(bone.pivot.x, "pivot_x", &AnimElement::PivotX, 1., ui, "X");
    //    });
    //});
    ui.horizontal(|ui| {
        label!("Rotation:", ui);
        let rot_el = &AnimElement::Rotation;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let input: egui::Response;
            let deg_mod = 180. / std::f32::consts::PI;
            input_response!(bone.rot, "rot", rot_el, deg_mod, ui, "", input);
            ui::draw_tutorial_rect(TutorialStep::EditBoneAnim, input.rect, shared, ui);
            if edited {
                shared
                    .ui
                    .start_next_tutorial_step(TutorialStep::PlayAnim, &shared.armature);
            }
        });
    });
    ui.horizontal(|ui| {
        label!("Z-Index:", ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            input!(bone.zindex, "zindex", &AnimElement::Zindex, 1., ui, "");
        });
    });

    if bone.vertices.len() == 0 {
        return;
    }

    // disabled: mesh deformation is unstable
    if true {
        return;
    }

    let mut mesh_label = "Edit Mesh";
    if shared.ui.editing_mesh {
        mesh_label = "Finish Edit";
    }

    ui.horizontal(|ui| {
        if ui.skf_button(mesh_label).clicked() {
            shared.ui.editing_mesh = !shared.ui.editing_mesh;
        }
    });

    if !shared.ui.editing_mesh {
        return;
    }

    ui.horizontal(|ui| {
        let tex_size = shared.armature.textures[bone.tex_idx as usize].size;
        if ui.skf_button("Center").clicked() {
            center_verts(&mut shared.selected_bone_mut().unwrap().vertices, &tex_size);
        }
        if ui.skf_button("Reset").clicked() {
            (
                shared.selected_bone_mut().unwrap().vertices,
                shared.selected_bone_mut().unwrap().indices,
            ) = renderer::create_tex_rect(&tex_size);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Base Index:")
            .on_hover_text("The vertex that all triangles point to");
        let base = bone.indices[0] as f32;
        let (edited, base, _) = ui.float_input("base_index".to_string(), shared, base, 1.);
        if edited {
            shared.selected_bone_mut().unwrap().indices = crate::renderer::setup_indices(
                &shared.selected_bone_mut().unwrap().vertices,
                base as i32,
            );
        }
    });
}

pub fn center_verts(verts: &mut Vec<Vertex>, tex_size: &Vec2) {
    let mut min = Vec2::default();
    let mut max = Vec2::default();
    for v in &mut *verts {
        if v.pos.x < min.x {
            min.x = v.pos.x;
        }
        if v.pos.y < min.y {
            min.y = v.pos.y
        }
        if v.pos.x > max.x {
            max.x = v.pos.x;
        }
        if v.pos.y > max.y {
            max.y = v.pos.y;
        }
    }

    let avg = (min + max) / 2.;
    for v in verts {
        v.pos -= avg;
        v.pos.x += tex_size.x / 2.;
        v.pos.y -= tex_size.y / 2.;
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_file_dialog(temp_img_path: String) {
    thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg", "tif"])
            .pick_file();
        if task == None {
            return;
        }
        create_temp_file(&temp_img_path, task.unwrap().as_path().to_str().unwrap());
    });
}
