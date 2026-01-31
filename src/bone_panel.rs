//! UI Bone window.

use crate::*;
use egui::IntoAtoms;
use std::path::PathBuf;
use ui::EguiUi;

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use crate::file_reader::*;
    pub use std::sync::Mutex;
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

pub fn draw(
    mut bone: Bone,
    ui: &mut egui::Ui,
    selections: &mut SelectionState,
    shared_ui: &mut crate::Ui,
    armature: &mut Armature,
    config: &Config,
    events: &mut EventState,
    input: &InputStates,
    edit_mode: &mut EditMode,
) {
    let sel = selections.clone();
    if shared_ui.dragging_bone
        || shared_ui.just_made_bone
        || armature.bones.len() == 0
        || sel.bone_idx > armature.bones.len() - 1
    {
        ui.disable();
        return;
    }

    ui.horizontal(|ui| {
        ui.heading(&shared_ui.loc("bone_panel.heading"));

        // delete label
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut col = config.colors.text;
            col -= Color::new(60, 60, 60, 0);
            let text = egui::RichText::new("🗑").size(15.).color(col);
            let hand = egui::CursorIcon::PointingHand;
            if ui.label(text).on_hover_cursor(hand).clicked() {
                let str = shared_ui.loc("polar.delete_bone").clone().to_string();
                let context_id = "b_".to_owned() + &armature.sel_bone(&sel).unwrap().id.to_string();
                shared_ui.context_menu.id = context_id;
                shared_ui.context_menu.keep = true;
                events.open_polar_modal(PolarId::DeleteBone, str);
            }
        });
    });

    ui.separator();
    ui.add_space(3.);

    ui.horizontal(|ui| {
        ui.label(&shared_ui.loc("bone_panel.name"));
        let sel_bone_name = armature.sel_bone(&sel).unwrap().name.clone();
        let (edited, value, _) = ui.text_input("Name".to_string(), shared_ui, sel_bone_name, None);
        if edited {
            armature.sel_bone_mut(&sel).unwrap().name = value;
        }
    });

    let mut edited = false;

    // Backbone of editable bone fields. Do not use by itself, instead refer to `input!`.
    macro_rules! check_input_edit {
        ($float:expr, $element:expr, $ui:expr, $label:expr) => {
            if edited {
                let mut anim_id = selections.anim;
                if !shared_ui.is_animating(&edit_mode, &selections) {
                    anim_id = usize::MAX;
                }

                let frame = selections.anim_frame;
                events.save_edited_bone(selections.bone_idx);
                events.edit_bone(bone.id, $element, $float, anim_id, frame);
                *shared_ui.saving.lock().unwrap() = shared::Saving::Autosaving;
            }
            if $label != "" {
                $ui.label($label);
            }
        };
    }

    // main macro to use for editable bone fields
    macro_rules! input {
        ($float:expr, $id:expr, $element:expr, $modifier:expr, $ui:expr, $label:expr) => {
            (edited, $float, _) =
                $ui.float_input($id.to_string(), shared_ui, $float, $modifier, None);
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

    let has_ik = !bone.ik_disabled && armature.bone_eff(bone.id) != JointEffector::None;
    let str_cant_edit = shared_ui
        .loc("bone_panel.inverse_kinematics.cant_edit")
        .clone();

    ui.add_enabled_ui(!has_ik, |ui| {
        ui.horizontal(|ui| {
            label!(&shared_ui.loc("bone_panel.position"), ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let pos_y = &AnimElement::PositionY;
                input!(bone.pos.y, "pos_y", pos_y, 1., ui, "Y");
                let pos_x = &AnimElement::PositionX;
                input!(bone.pos.x, "pos_x", pos_x, 1., ui, "X");
            })
        });

        ui.horizontal(|ui| {
            label!(&shared_ui.loc("bone_panel.scale"), ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                input!(bone.scale.y, "scale_y", &AnimElement::ScaleY, 1., ui, "H");
                input!(bone.scale.x, "scale_x", &AnimElement::ScaleX, 1., ui, "W");
            });
        });
    })
    .response
    .on_disabled_hover_text(str_cant_edit);

    let not_end_ik = !has_ik || armature.bone_eff(bone.id) == JointEffector::End;
    let str_cant_edit = shared_ui
        .loc("bone_panel.inverse_kinematics.cant_edit")
        .clone();

    ui.add_enabled_ui(not_end_ik, |ui| {
        ui.horizontal(|ui| {
            label!(&shared_ui.loc("bone_panel.rotation"), ui);
            let rot_el = &AnimElement::Rotation;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let deg_mod = 180. / std::f32::consts::PI;
                input!(bone.rot, "rot", rot_el, deg_mod, ui, "");
            });
        });
    })
    .response
    .on_disabled_hover_text(str_cant_edit);

    let mut children = vec![];
    armature_window::get_all_children(&armature.bones, &mut children, &bone);
    let parents = armature.get_all_parents(bone.id);

    ui.add_space(20.);

    // show 'IK root bone' button if this is a target bone
    let is_target_of = armature
        .bones
        .iter()
        .position(|b| b.ik_family_id != -1 && b.ik_target_id == bone.id);
    if is_target_of != None {
        let target_str = shared_ui.loc("bone_panel.target_bone").to_owned();
        ui.label(target_str + &armature.bones[is_target_of.unwrap()].name + ".");
        if ui.skf_button("Go to IK bone").clicked() {
            events.select_bone(is_target_of.unwrap(), false);
        };
        ui.add_space(20.);
    }

    #[rustfmt::skip]
    texture_effects(ui, &mut bone, shared_ui, &selections, config, &edit_mode, &input, armature, events);
    ui.add_space(20.);

    if children.len() == 0 && parents.len() == 0 && bone.tex == "" {
        ui.add_space(10.);
        let mut cache = egui_commonmark::CommonMarkCache::default();
        let loc = shared_ui.loc("bone_panel.bone_empty").to_string();
        let str = utils::markdown(loc, shared_ui.local_doc_url.to_string());
        //egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
    }

    if bone.vertices.len() > 0 && armature.tex_of(bone.id) != None {
        mesh_deformation(
            ui, &bone, shared_ui, events, config, selections, armature, edit_mode,
        );
        ui.add_space(20.);
    }

    if children.len() > 0 || parents.len() > 0 {
        inverse_kinematics(ui, &bone, selections, shared_ui, config, armature, events);
        if !bone.ik_folded {
            ui.add_space(20.);
        }
    }

    ui.add_space(20.);
}

pub fn inverse_kinematics(
    ui: &mut egui::Ui,
    bone: &Bone,
    selections: &mut SelectionState,
    shared_ui: &mut crate::Ui,
    config: &Config,
    armature: &Armature,
    events: &mut EventState,
) {
    let sel = selections.clone();
    let str_heading = &shared_ui
        .loc("bone_panel.inverse_kinematics.heading")
        .clone();
    let str_desc = &shared_ui.loc("bone_panel.inverse_kinematics.desc").clone();
    let frame = egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin {
            bottom: 5,
            top: 5,
            left: 5,
            right: 5,
        });

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading.to_owned()).on_hover_text(str_desc);
            let color = config.colors.inverse_kinematics;
            ui.label(egui::RichText::new("🔧").size(16.).color(color));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fold_icon = if bone.ik_folded { "⏴" } else { "⏷" };
                let pointing_hand = egui::CursorIcon::PointingHand;
                if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                    let ik_folded = armature.sel_bone(&sel).unwrap().ik_folded;
                    events.toggle_ik_folded(if ik_folded { 0 } else { 1 });
                }

                if armature.bone_eff(bone.id) == JointEffector::None {
                    return;
                }

                let mut is_root = true;
                if armature.bone_eff(bone.id) != JointEffector::Start {
                    is_root = false;
                }

                let mut enabled = !bone.ik_disabled;
                let str_desc = &shared_ui.loc("bone_panel.inverse_kinematics.enabled_desc");
                //let checkbox = ui
                //    .add_enabled(is_root, egui::Checkbox::new(&mut enabled, "".into_atoms()))
                //    .on_hover_text(str_desc);
                //if checkbox.clicked() {
                //    let mut bones = vec![];
                //    armature_window::get_all_children(&armature.bones, &mut bones, &bone);
                //    bones.push(bone.clone());
                //    for b in 0..bones.len() {
                //        let disabled = armature.bones[b].ik_disabled;
                //        events.toggle_bone_ik_disabled(b, !disabled);
                //    }

                //    if is_root {
                //        return;
                //    }

                //    // emable parents IK as well

                //    let parents = armature.get_all_parents(bone.id);
                //    for p in parents {
                //        if armature.bone_eff(p.id) == JointEffector::None {
                //            continue;
                //        }

                //        let idx = armature.bones.iter().position(|b| b.id == p.id).unwrap();
                //        events.toggle_bone_ik_disabled(idx, false);
                //    }
                //}
            })
        });
    });
    ui.add_space(2.5);

    if bone.ik_folded {
        return;
    }

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("bone_panel.inverse_kinematics.family_index"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_selected = if bone.ik_family_id == -1 {
                shared_ui.loc("none").to_string()
            } else {
                bone.ik_family_id.to_string()
            };
            egui::ComboBox::new("joint_eff", "")
                .selected_text(str_selected)
                .width(40.)
                .show_ui(ui, |ui| {
                    let mut selected = -1;

                    let mut ik_family_ids = vec![];
                    for bone in &armature.bones {
                        if !ik_family_ids.contains(&bone.ik_family_id) && bone.ik_family_id != -1 {
                            ik_family_ids.push(bone.ik_family_id);
                        }
                    }

                    ui.selectable_value(&mut selected, -3, "None");
                    for id in &ik_family_ids {
                        ui.selectable_value(&mut selected, *id, id.to_string());
                    }
                    ui.selectable_value(&mut selected, -2, "New");

                    if selected != -1 {
                        let mut id = selected;
                        if selected == -2 {
                            id = generate_id(ik_family_ids);
                        } else if selected == -3 {
                            id = -1;
                        }
                        let sel = &selections;
                        let family_id = AnimElement::IkFamilyId;
                        events.edit_bone(bone.id, &family_id, id as f32, sel.anim, sel.anim_frame);
                    }
                })
                .response
                .on_hover_text(&shared_ui.loc("bone_panel.inverse_kinematics.effector_desc"));
        });
    });

    if bone.ik_family_id == -1 {
        return;
    }

    let bones = &armature.bones;
    let ik_id = bone.ik_family_id;
    let root_id = bones.iter().find(|b| b.ik_family_id == ik_id).unwrap().id;

    let go_to_root_str = shared_ui.loc("bone_panel.inverse_kinematics.go_to_root");
    if root_id != bone.id {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.skf_button(&go_to_root_str).clicked() {
                    let idx = bones.iter().position(|b| b.id == root_id).unwrap();
                    events.select_bone(idx, false);
                    selections.bone_ids = vec![];
                }
            });
        });
        return;
    }

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("bone_panel.inverse_kinematics.mode_label"));
        let mode = armature.sel_bone(&sel).unwrap().ik_mode;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::new("ik_mode", "")
                .selected_text(mode.to_string())
                .width(40.)
                .show_ui(ui, |ui| {
                    let mut selected_mode = -1;
                    ui.selectable_value(&mut selected_mode, 0, "FABRIK");
                    ui.selectable_value(&mut selected_mode, 1, "Arc");
                    if selected_mode != -1 {
                        events.edit_bone(
                            bone.id,
                            &AnimElement::IkMode,
                            selected_mode as f32,
                            selections.anim,
                            selections.anim_frame,
                        );
                    }
                })
                .response
                .on_hover_text(str_desc);
        });
    });

    ui.horizontal(|ui| {
        let ik = "bone_panel.inverse_kinematics.";
        ui.label(&shared_ui.loc(&(ik.to_owned() + "constraint")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_none = &shared_ui.loc("none").clone();
            let str_clockwise = shared_ui.loc(&(ik.to_owned() + "Clockwise")).clone() + "  ⟳";
            let str_ccw = shared_ui.loc(&(ik.to_owned() + "CounterClockwise")).clone() + "  ⟲";
            let str_desc = &shared_ui.loc(&(ik.to_owned() + "constraint_desc")).clone();
            let selected = match bone.ik_constraint {
                JointConstraint::Clockwise => str_clockwise.clone(),
                JointConstraint::CounterClockwise => str_ccw.clone(),
                _ => str_none.clone(),
            };

            egui::ComboBox::new("joint_constraint", "")
                .selected_text(selected)
                .width(40.)
                .show_ui(ui, |ui| {
                    let mut ik = armature.sel_bone(&sel).unwrap().ik_constraint.clone();
                    ui.selectable_value(&mut ik, JointConstraint::None, str_none);
                    ui.selectable_value(&mut ik, JointConstraint::Clockwise, str_clockwise);
                    ui.selectable_value(&mut ik, JointConstraint::CounterClockwise, str_ccw);
                    if ik != armature.sel_bone(&sel).unwrap().ik_constraint {
                        let sel = selections.anim;
                        let frame = selections.anim_frame;
                        let constraint = AnimElement::IkConstraint;
                        events.edit_bone(bone.id, &constraint, (ik as usize) as f32, sel, frame);
                    }
                })
                .response
                .on_hover_text(str_desc);
        });
    });

    let target_buttons_width = 60.;

    ui.horizontal(|ui| {
        ui.label(&shared_ui.loc("bone_panel.inverse_kinematics.target"));

        let bone_id = bone.ik_target_id;
        if let Some(target) = armature.bones.iter().find(|b| b.id == bone_id) {
            let width = ui.available_width();
            let tr_name = utils::trunc_str(ui, &target.name.clone(), width - target_buttons_width);
            if ui.selectable_label(false, tr_name).clicked() {
                let bones = &armature.bones;
                let ik_id = bone.ik_target_id;
                let idx = bones.iter().position(|b| b.id == ik_id).unwrap();
                events.select_bone(idx, false);
            };
        } else {
            ui.label(shared_ui.loc("none"));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_set_target_raw = "bone_panel.inverse_kinematics.set_target";
            let str_set_target = shared_ui.loc(&str_set_target_raw).clone();
            let str_remove_target_raw = "bone_panel.inverse_kinematics.remove_target";
            let str_remove_target = shared_ui.loc(&str_remove_target_raw).clone();

            let remove_enabled = bone.ik_target_id != -1;
            ui.add_enabled_ui(remove_enabled, |ui| {
                let button = ui.skf_button("🗑");
                if button.on_hover_text(str_remove_target).clicked() {
                    events.remove_ik_target();
                }
            });

            if ui.skf_button("⌖").on_hover_text(str_set_target).clicked() {
                events.toggle_setting_ik_target(1);
            }
        });
    });
}

pub fn mesh_deformation(
    ui: &mut egui::Ui,
    bone: &Bone,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    config: &Config,
    selections: &mut SelectionState,
    armature: &mut Armature,
    edit_mode: &mut EditMode,
) {
    let str_heading = &shared_ui.loc("bone_panel.mesh_deformation.heading").clone();
    let str_desc = &shared_ui.loc("bone_panel.mesh_deformation.desc").clone();

    let sel = selections.clone();

    let frame = egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading.to_owned()).on_hover_text(str_desc);
            let color = config.colors.meshdef;
            ui.label(egui::RichText::new("⬟").color(color));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fold_icon = if bone.meshdef_folded { "⏴" } else { "⏷" };
                let pointing_hand = egui::CursorIcon::PointingHand;
                if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                    let meshdef = armature.sel_bone(&sel).unwrap().meshdef_folded;
                    events.toggle_meshdef_folded(if meshdef { 0 } else { 1 });
                }
            })
        });
    });
    ui.add_space(2.5);

    if bone.meshdef_folded {
        return;
    }

    // check if this bone is a weight
    let parents = armature.get_all_parents(bone.id);
    let mut mesh_parent_id = -1;
    'parent: for parent in parents {
        for bind in parent.binds {
            if bind.bone_id == bone.id {
                mesh_parent_id = parent.id;
                break 'parent;
            }
        }
    }
    if mesh_parent_id != -1 {
        let str = &shared_ui
            .loc("bone_panel.mesh_deformation.go_to_mesh")
            .clone();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.skf_button(str).clicked() {
                    let bones = &armature.bones;
                    let idx = bones.iter().position(|b| b.id == mesh_parent_id).unwrap();
                    events.select_bone(idx, false);
                }
            });
        });

        return;
    }

    if armature.tex_of(bone.id) == None {
        return;
    }

    let str_edit = &shared_ui.loc("bone_panel.mesh_deformation.edit").clone();
    let str_finish_edit = &shared_ui
        .loc("bone_panel.mesh_deformation.finish_edit")
        .clone();
    let mut mesh_label = str_edit;
    if edit_mode.showing_mesh {
        mesh_label = str_finish_edit;
    }

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button(&mesh_label).clicked() {
                events.toggle_showing_mesh(if edit_mode.showing_mesh { 0 } else { 1 });
            }
        });
    });

    ui.add_enabled_ui(edit_mode.showing_mesh, |ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let str_reset = &shared_ui.loc("bone_panel.mesh_deformation.reset");
                let str_reset_desc = &shared_ui.loc("bone_panel.mesh_deformation.reset_desc");
                let can_reset = !edit_mode.setting_bind_verts;
                let button = ui
                    .add_enabled(can_reset, egui::Button::new(str_reset))
                    .on_hover_text(str_reset_desc);
                if button.clicked() {
                    events.reset_vertices();
                }

                let str_center = &shared_ui.loc("bone_panel.mesh_deformation.center");
                let str_center_desc = &shared_ui.loc("bone_panel.mesh_deformation.center_desc");
                let button = ui.skf_button(str_center);
                if button.on_hover_text(str_center_desc).clicked() {
                    events.center_bone_verts();
                }

                let trace_str = &shared_ui.loc("bone_panel.mesh_deformation.trace");
                if ui.skf_button(trace_str).clicked() {
                    events.trace_bone_verts();
                }
            });
        });
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("bone_panel.mesh_deformation.binds_label"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let headline = if selections.bind == -1 {
                shared_ui.loc("none").to_string()
            } else {
                selections.bind.to_string()
            };
            let combo_box = egui::ComboBox::new("bone_weights", "").selected_text(headline);
            combo_box.show_ui(ui, |ui| {
                let mut selected_value: i32 = -1;
                for b in 0..bone.binds.len() {
                    ui.selectable_value(&mut selected_value, b as i32, b.to_string());
                }
                ui.selectable_value(&mut selected_value, -2, shared_ui.loc("new_option"));

                events.select_bind(selected_value);
            });
        });
    });

    if selections.bind == -1 {
        return;
    }

    let binds = armature.sel_bone(&sel).unwrap().binds.clone();
    if selections.bind as usize > binds.len() - 1 {
        selections.bind = -1;
        return;
    }

    ui.horizontal(|ui| {
        let bone_id = binds[selections.bind as usize].bone_id;
        let mut bone_name = shared_ui.loc("none").to_string();
        if let Some(bone) = armature.bones.iter().find(|bone| bone.id == bone_id) {
            bone_name = bone.name.clone();
        }
        ui.label(shared_ui.loc("bone_panel.mesh_deformation.bone_label") + &bone_name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_set_bone = if edit_mode.setting_bind_bone {
                shared_ui.loc("bone_panel.mesh_deformation.finish")
            } else {
                shared_ui.loc("bone_panel.mesh_deformation.bone_set")
            };
            if ui.skf_button(&str_set_bone).clicked() {
                edit_mode.setting_bind_bone = !edit_mode.setting_bind_bone;
            }
        });
    });
    let selected = selections.bind as usize;

    if binds[selected].bone_id == -1 {
        return;
    }

    let vert_id_len = armature.sel_bone(&sel).unwrap().binds[selected].verts.len();
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_set_verts = if edit_mode.setting_bind_verts {
                shared_ui.loc("bone_panel.mesh_deformation.finish")
            } else {
                shared_ui.loc("bone_panel.mesh_deformation.bind_verts")
            };
            if ui.skf_button(&str_set_verts).clicked() {
                events.toggle_binding_verts(if edit_mode.setting_bind_verts { 1 } else { 0 });
            }
        });
    });

    ui.horizontal(|ui| {
        if vert_id_len > 0 {
            ui.label(shared_ui.loc("bone_panel.mesh_deformation.weights_label"));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let binding_verts = edit_mode.setting_bind_verts;
            if binding_verts {
                ui.add_enabled_ui(false, |ui| {
                    ui.checkbox(&mut shared_ui.was_editing_path, "".into_atoms());
                });
            } else {
                let bind = &armature.sel_bone(&sel).unwrap().binds[selected];
                let mut new_path = bind.is_path;
                ui.checkbox(&mut new_path, "".into_atoms());
                if new_path != bind.is_path {
                    events.toggle_bind_pathing(selected, new_path);
                }
            }

            ui.label(shared_ui.loc("bone_panel.mesh_deformation.pathing_label"))
                .on_hover_text(shared_ui.loc("bone_panel.mesh_deformation.pathing_desc"));
        });
    });

    let selected = selections.bind;
    let bind = armature.sel_bone(&sel).unwrap().binds[selected as usize].clone();
    if bind.verts.len() == 0 {
        ui.label(shared_ui.loc("bone_panel.mesh_deformation.no_bound_verts"));
    } else {
        for w in 0..bind.verts.len() {
            ui.horizontal(|ui| {
                let str_label = bind.verts[w].id.to_string() + ":";
                ui.label(str_label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut new_weight = bind.verts[w].weight;
                    ui.add(egui::Slider::new(&mut new_weight, (0.)..=1.));
                    if new_weight != bind.verts[w].weight {
                        events.set_bind_weight(w, new_weight);
                    }
                });
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_file_dialog(file_path: &Arc<Mutex<Vec<PathBuf>>>, file_type: &Arc<Mutex<i32>>) {
    let filepath = Arc::clone(file_path);
    let filetype = Arc::clone(file_type);
    thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg", "tif"])
            .pick_files();
        if task == None {
            return;
        }
        *filepath.lock().unwrap() = task.unwrap();
        *filetype.lock().unwrap() = 1;
    });
}

pub fn texture_effects(
    ui: &mut egui::Ui,
    bone: &mut Bone,
    shared_ui: &mut crate::Ui,
    selections: &SelectionState,
    config: &Config,
    edit_mode: &EditMode,
    input: &InputStates,
    armature: &Armature,
    events: &mut EventState,
) {
    let str_heading = &shared_ui.loc("bone_panel.texture_effects.heading").clone();
    let frame = egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading.to_owned());
            let color = config.colors.texture;
            ui.label(egui::RichText::new("🖻").color(color));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fold_icon = if bone.effects_folded { "⏴" } else { "⏷" };
                let pointing_hand = egui::CursorIcon::PointingHand;
                if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                    let effects = bone.effects_folded;
                    events.toggle_effects_folded(if effects { 0 } else { 1 });
                }
            })
        });
    });
    ui.add_space(2.5);

    if bone.effects_folded {
        return;
    }

    let tex = armature.anim_tex_of(bone.id);

    let tex_name_col = if tex != None {
        config.colors.text
    } else {
        config.colors.light_accent + Color::new(60, 60, 60, 0)
    };

    let mut selected_tex = bone.tex.clone();
    let mut tex_name = if bone.tex != "" {
        bone.tex.clone()
    } else {
        shared_ui.loc("none")
    };
    ui.horizontal(|ui| {
        ui.label(&shared_ui.loc("bone_panel.texture"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            tex_name = utils::trunc_str(ui, &tex_name, 100.);
            let combo_box = egui::ComboBox::new("tex_selector", "")
                .width(100.)
                .selected_text(egui::RichText::new(tex_name).color(tex_name_col));
            combo_box.show_ui(ui, |ui| {
                let mut texes = vec![];
                for style in &armature.styles {
                    let textures = style.textures.iter();
                    let mut names = textures.map(|t| t.name.clone()).collect::<Vec<String>>();
                    texes.append(&mut names);
                }

                // remove duplicates
                texes.sort_unstable();
                texes.dedup();

                ui.selectable_value(&mut selected_tex, "".to_string(), "[None]");
                for tex in texes {
                    let name = utils::trunc_str(ui, &tex.clone(), ui.min_rect().width());
                    ui.selectable_value(&mut selected_tex, tex.clone(), &name);
                }
                ui.selectable_value(&mut selected_tex, "[Setup]".to_string(), "[Setup]");
            });
        });
    });

    if selected_tex == "[Setup]" {
        shared_ui.styles_modal = true;
    } else if selected_tex != bone.tex {
        events.set_bone_texture(bone.id as usize, selected_tex);
    }

    ui.horizontal(|ui| {
        ui.label("Tint: ");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let og_col: [f32; 4] = [bone.tint.r, bone.tint.g, bone.tint.b, bone.tint.a];
            let mut col = og_col.clone();
            ui.color_edit_button_rgba_premultiplied(&mut col);
            if col == og_col || !input.left_down || !egui::Popup::is_any_open(ui.ctx()) {
                return;
            }
            let anim_id = if edit_mode.anim_open {
                selections.anim
            } else {
                usize::MAX
            };
            let frame = selections.anim_frame;
            events.edit_bone(bone.id, &AnimElement::TintR, col[0], anim_id, frame);
            events.edit_bone(bone.id, &AnimElement::TintG, col[1], anim_id, frame);
            events.edit_bone(bone.id, &AnimElement::TintB, col[2], anim_id, frame);
            events.edit_bone(bone.id, &AnimElement::TintA, col[3], anim_id, frame);
        });
    });

    ui.horizontal(|ui| {
        ui.label(&shared_ui.loc("bone_panel.zindex"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let zindex = bone.zindex as f32;
            let (edited, value, _) =
                ui.float_input("zindex".to_string(), shared_ui, zindex, 1., None);
            if edited {
                let el = &AnimElement::Zindex;
                events.save_edited_bone(selections.bone_idx);
                events.edit_bone(bone.id, el, value, selections.anim, selections.anim_frame);
            }
        });
    });
}
