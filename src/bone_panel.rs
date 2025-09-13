//! UI Bone window.

use crate::*;
use egui::IntoAtoms;
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

pub fn draw(mut bone: Bone, ui: &mut egui::Ui, shared: &mut Shared) {
    if shared.ui.has_state(UiState::DraggingBone) {
        ui.disable();
        return;
    }

    ui.horizontal(|ui| {
        ui.heading(shared.loc("bone_panel.heading"));

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
                let str = shared.loc("polar.delete_bone").clone();
                shared.ui.context_menu.id = shared.selected_bone().unwrap().id;
                shared.ui.open_polar_modal(PolarId::DeleteBone, &str);
            }
        });
    });

    ui.separator();
    ui.add_space(3.);

    ui.horizontal(|ui| {
        ui.label(shared.loc("bone_panel.name"));
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
        shared.loc("bone_panel.texture_set_none").to_string()
    } else {
        shared.armature.texture_sets[bone.tex_set_idx as usize]
            .name
            .to_string()
    };

    let mut selected_set = bone.tex_set_idx;
    ui.horizontal(|ui| {
        ui.label(shared.loc("bone_panel.texture_set"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::new("mod", "")
                .selected_text(set_name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut selected_set,
                        -1,
                        shared.loc("bone_panel.texture_set_none"),
                    );
                    let sets = &shared.armature.texture_sets;
                    for s in 0..sets.len() {
                        if sets[s].textures.len() == 0 {
                            continue;
                        }
                        ui.selectable_value(&mut selected_set, s as i32, sets[s].name.clone());
                    }
                    ui.selectable_value(
                        &mut selected_set,
                        -2,
                        shared.loc("bone_panel.texture_set_setup"),
                    );
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

    if shared.armature.is_valid_tex(bone.id) {
        let mut selected_tex = bone.tex_idx;
        let tex_name = &shared.armature.texture_sets[bone.tex_set_idx as usize].textures
            [bone.tex_idx as usize]
            .name;
        let str_idx = bone.tex_idx.to_string() + ") ";
        ui.horizontal(|ui| {
            ui.label(shared.loc("bone_panel.texture_index"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::new("tex_selector", "")
                    .selected_text(str_idx + tex_name)
                    .show_ui(ui, |ui| {
                        let set = &shared.armature.texture_sets[bone.tex_set_idx as usize];
                        for t in 0..set.textures.len() {
                            let str_idx = t.to_string() + ") ";
                            ui.selectable_value(
                                &mut selected_tex,
                                t as i32,
                                str_idx + &set.textures[t].name.clone(),
                            );
                        }
                        ui.selectable_value(
                            &mut selected_tex,
                            -2,
                            shared.loc("bone_panel.texture_set_setup"),
                        );
                    })
                    .response;
            });
        });

        if selected_tex == -2 {
            shared.ui.selected_tex_set_idx = bone.tex_set_idx;
            shared.ui.set_state(UiState::ImageModal, true);
        } else if selected_tex != bone.tex_idx {
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

    // Backbone of editable bone fields. Do not use by itself, instead refer to `input!`.
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
                shared.saving = shared::Saving::Autosaving;
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

    // for labels that are not part of any input fields (eg "Position:", "Rotation:", etc)
    macro_rules! label {
        ($name:expr, $ui:expr) => {
            $ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label($name);
            });
        };
    }

    let has_ik = !bone.ik_disabled && bone.joint_effector != JointEffector::None;
    let str_cant_edit = shared
        .loc("bone_panel.inverse_kinematics.cant_edit")
        .clone();

    ui.add_enabled_ui(!has_ik, |ui| {
        ui.horizontal(|ui| {
            label!(shared.loc("bone_panel.position"), ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let pos_y = &AnimElement::PositionY;
                input!(bone.pos.y, "pos_y", pos_y, 1., ui, "Y");

                let pos_x = &AnimElement::PositionX;
                input!(bone.pos.x, "pos_x", pos_x, 1., ui, "X");
            })
        });

        ui.horizontal(|ui| {
            label!(shared.loc("bone_panel.scale"), ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                input!(bone.scale.y, "scale_y", &AnimElement::ScaleY, 1., ui, "H");
                input!(bone.scale.x, "scale_x", &AnimElement::ScaleX, 1., ui, "W");
            });
        });
        ui.horizontal(|ui| {
            label!(shared.loc("bone_panel.rotation"), ui);
            let rot_el = &AnimElement::Rotation;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let deg_mod = 180. / std::f32::consts::PI;
                input!(bone.rot, "rot", rot_el, deg_mod, ui, "");
            });
        });
    })
    .response
    .on_disabled_hover_text(str_cant_edit);

    ui.horizontal(|ui| {
        label!(shared.loc("bone_panel.zindex"), ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut zindex = bone.zindex as f32;
            input!(zindex, "zindex", &AnimElement::Zindex, 1., ui, "");
            bone.zindex = zindex as i32;
        });
    });

    // disabled: inverse kinematics (not ready)
    // disabled: mesh deformation (not ready either)
    if true {
        return;
    }

    let mut children = vec![];
    armature_window::get_all_children(&shared.armature.bones, &mut children, &bone);
    let parents = shared.armature.get_all_parents(bone.id);

    let section_spacing = 10.;

    if children.len() > 0 || parents.len() > 0 {
        ui.add_space(section_spacing);
        inverse_kinematics(ui, shared, &bone);
    }

    if bone.vertices.len() == 0 || selected_set == -1 {
        return;
    }

    mesh_deformation(ui, shared, &bone);
}

pub fn inverse_kinematics(ui: &mut egui::Ui, shared: &mut Shared, bone: &Bone) {
    let str_heading = shared.loc("bone_panel.inverse_kinematics.heading").clone();
    let str_desc = shared.loc("bone_panel.inverse_kinematics.desc").clone();
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(str_heading.to_owned() + ICON_INFO)
            .on_hover_text(str_desc);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let fold_icon = if bone.ik_folded { "⏴" } else { "⏷" };
            let pointing_hand = egui::CursorIcon::PointingHand;
            if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                shared.selected_bone_mut().unwrap().ik_folded =
                    !shared.selected_bone_mut().unwrap().ik_folded;
            }

            if bone.joint_effector == JointEffector::Start {
                let mut enabled = !bone.ik_disabled;
                let str_desc = shared.loc("bone_panel.inverse_kinematics.enabled_desc");
                let checkbox = ui
                    .checkbox(&mut enabled, "".into_atoms())
                    .on_hover_text(str_desc);
                if checkbox.clicked() {
                    let mut bones = vec![];
                    armature_window::get_all_children(&shared.armature.bones, &mut bones, &bone);
                    bones.push(bone.clone());
                    for bone in bones {
                        shared.armature.find_bone_mut(bone.id).unwrap().ik_disabled = !enabled;
                    }
                }
            }
        })
    });

    ui.separator();

    if bone.ik_folded {
        return;
    }

    ui.horizontal(|ui| {
        ui.label(shared.loc("bone_panel.inverse_kinematics.effector"));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_selected = shared
                .loc(
                    &("bone_panel.inverse_kinematics.".to_owned()
                        + &bone.joint_effector.to_string()),
                )
                .clone();
            let str_none = shared.loc("bone_panel.inverse_kinematics.None").clone();
            let str_start = shared.loc("bone_panel.inverse_kinematics.Start").clone();
            let str_middle = shared.loc("bone_panel.inverse_kinematics.Middle").clone();
            let str_end = shared.loc("bone_panel.inverse_kinematics.End").clone();
            egui::ComboBox::new("joint_eff", "")
                .selected_text(str_selected)
                .width(40.)
                .show_ui(ui, |ui| {
                    let bone = &mut shared.selected_bone_mut().unwrap().joint_effector;
                    ui.selectable_value(bone, JointEffector::None, str_none);
                    ui.selectable_value(bone, JointEffector::Start, str_start);
                    ui.selectable_value(bone, JointEffector::Middle, str_middle);
                    ui.selectable_value(bone, JointEffector::End, str_end);
                })
                .response
                .on_hover_text(shared.loc("bone_panel.inverse_kinematics.effector_desc"));
        });
    });

    if bone.joint_effector == JointEffector::Start {
        let icon: &str;
        let const_label = if bone.constraint == JointConstraint::CounterClockwise {
            icon = "  ⟲";
            "CCW".to_string()
        } else {
            icon = "  ⟳";
            bone.constraint.to_string()
        };
        ui.horizontal(|ui| {
            ui.label(shared.loc("bone_panel.inverse_kinematics.constraint"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let last_constraint = bone.clone().constraint;
                let str_selected = shared
                    .loc(&("bone_panel.inverse_kinematics.".to_owned() + &const_label))
                    .to_owned()
                    + icon;
                let str_none = shared.loc("bone_panel.inverse_kinematics.None").clone();
                let str_clockwise = shared
                    .loc("bone_panel.inverse_kinematics.Clockwise")
                    .clone();
                let str_ccw = shared.loc("bone_panel.inverse_kinematics.CCW").clone();
                let str_desc = shared
                    .loc("bone_panel.inverse_kinematics.constraint_desc")
                    .clone();
                egui::ComboBox::new("joint_constraint", "")
                    .selected_text(str_selected)
                    .width(40.)
                    .show_ui(ui, |ui| {
                        let constraint = &mut shared.selected_bone_mut().unwrap().constraint;
                        ui.selectable_value(constraint, JointConstraint::None, str_none);
                        ui.selectable_value(
                            constraint,
                            JointConstraint::Clockwise,
                            str_clockwise + "  ⟳",
                        );
                        ui.selectable_value(
                            constraint,
                            JointConstraint::CounterClockwise,
                            str_ccw + "  ⟲",
                        );
                    })
                    .response
                    .on_hover_text(str_desc);

                if last_constraint == shared.selected_bone().unwrap().constraint {
                    return;
                }

                let mut joints = vec![];
                armature_window::get_all_children(&shared.armature.bones, &mut joints, &bone);
                joints = joints
                    .iter()
                    .filter(|joint| joint.joint_effector != JointEffector::None)
                    .cloned()
                    .collect();
                for joint in joints {
                    shared
                        .armature
                        .bones
                        .iter_mut()
                        .find(|bone| bone.id == joint.id)
                        .unwrap()
                        .constraint = shared.selected_bone().unwrap().constraint;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(shared.loc("bone_panel.inverse_kinematics.target"));

            if let Some(target) = shared.armature.find_bone(bone.ik_target_id) {
                if ui.clickable_label(target.name.clone()).clicked() {
                    shared.ui.selected_bone_idx =
                        shared.armature.find_bone_idx(bone.ik_target_id).unwrap();
                };
            } else {
                ui.label("None");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let str_set_target = shared
                    .loc("bone_panel.inverse_kinematics.set_target")
                    .clone();
                let str_remove_target = shared
                    .loc("bone_panel.inverse_kinematics.remove_target")
                    .clone();

                let remove_enabled = bone.ik_target_id != -1;
                ui.add_enabled_ui(remove_enabled, |ui| {
                    if ui
                        .skf_button("🗑")
                        .on_hover_text(str_remove_target)
                        .clicked()
                    {
                        shared.selected_bone_mut().unwrap().ik_target_id = -1;
                    }
                });

                if ui.skf_button("⌖").on_hover_text(str_set_target).clicked() {
                    shared.ui.setting_ik_target = true;
                }
            });
        });
    }

    ui.add_space(20.);
}

pub fn mesh_deformation(ui: &mut egui::Ui, shared: &mut Shared, bone: &Bone) {
    let str_heading = shared.loc("bone_panel.mesh_deformation.heading").clone();
    let str_desc = shared.loc("bone_panel.mesh_deformation.desc").clone();
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(str_heading.to_owned() + ICON_INFO)
            .on_hover_text(str_desc);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let fold_icon = if bone.meshdef_folded { "⏴" } else { "⏷" };
            let pointing_hand = egui::CursorIcon::PointingHand;
            if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                shared.selected_bone_mut().unwrap().meshdef_folded =
                    !shared.selected_bone_mut().unwrap().meshdef_folded;
            }
        })
    });
    ui.separator();

    if bone.meshdef_folded {
        return;
    }

    let str_edit = shared.loc("bone_panel.mesh_deformation.edit").clone();
    let str_finish_edit = shared
        .loc("bone_panel.mesh_deformation.finish_edit")
        .clone();
    let mut mesh_label = str_edit;
    if shared.ui.editing_mesh {
        mesh_label = str_finish_edit;
    }

    ui.horizontal(|ui| {
        if ui.skf_button(&mesh_label).clicked() {
            shared.ui.editing_mesh = !shared.ui.editing_mesh;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let tex_size = shared.armature.texture_sets[bone.tex_set_idx as usize].textures
                [bone.tex_idx as usize]
                .size
                .clone();
            let str_center = shared.loc("bone_panel.mesh_deformation.center");
            let str_center_desc = shared.loc("bone_panel.mesh_deformation.center_desc");
            if ui
                .skf_button(str_center)
                .on_hover_text(str_center_desc)
                .clicked()
            {
                center_verts(&mut shared.selected_bone_mut().unwrap().vertices, &tex_size);
            }
            let str_reset = shared.loc("bone_panel.mesh_deformation.reset");
            let str_reset_desc = shared.loc("bone_panel.mesh_deformation.reset_desc");
            if ui
                .skf_button(str_reset)
                .on_hover_text(str_reset_desc)
                .clicked()
            {
                let (verts, indices) = renderer::create_tex_rect(&tex_size);
                shared.selected_bone_mut().unwrap().vertices = verts;
                shared.selected_bone_mut().unwrap().indices = indices;
            }

            // disbaled: polygonation not great yet
            //
            //if ui.skf_button("Generate").clicked() {
            //    let (verts, indices) = renderer::polygonate(
            //        &shared.armature.texture_sets[bone.tex_set_idx as usize].textures
            //            [bone.tex_idx as usize]
            //            .image,
            //    );
            //    shared.selected_bone_mut().unwrap().vertices = verts;
            //    shared.selected_bone_mut().unwrap().indices = indices;
            //}
        });
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
