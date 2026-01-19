//! UI Bone window.

use crate::*;
use egui::IntoAtoms;
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

pub fn draw(mut bone: Bone, ui: &mut egui::Ui, shared: &mut Shared) {
    if shared.ui.dragging_bone || shared.ui.just_made_new_bone {
        ui.disable();
        return;
    }

    ui.horizontal(|ui| {
        ui.heading(&shared.loc("bone_panel.heading"));

        // delete label
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut col = shared.config.colors.text;
            col -= Color::new(60, 60, 60, 0);
            let text = egui::RichText::new("🗑").size(15.).color(col);
            let hand = egui::CursorIcon::PointingHand;
            if ui.label(text).on_hover_cursor(hand).clicked() {
                let str = shared.loc("polar.delete_bone").clone().to_string();
                let context_id =
                    "bone_".to_owned() + &shared.selected_bone().unwrap().id.to_string();
                shared.ui.context_menu.id = context_id;
                shared.ui.open_polar_modal(PolarId::DeleteBone, str);
            }
        });
    });

    ui.separator();
    ui.add_space(3.);

    ui.horizontal(|ui| {
        ui.label(&shared.loc("bone_panel.name"));
        let sel_bone_name = shared.selected_bone().unwrap().name.clone();
        let (edited, value, _) = ui.text_input("Name".to_string(), shared, sel_bone_name, None);
        if edited {
            shared.selected_bone_mut().unwrap().name = value;
        }
    });

    let tex = shared.armature.tex_of(bone.id);

    let tex_name_col = if tex != None {
        shared.config.colors.text
    } else {
        shared.config.colors.light_accent + Color::new(60, 60, 60, 0)
    };

    let mut selected_tex = bone.tex.clone();
    let mut tex_name = if tex != None {
        bone.tex.clone()
    } else {
        shared.loc("none")
    };
    ui.horizontal(|ui| {
        ui.label(&shared.loc("bone_panel.texture"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            tex_name = utils::trunc_str(ui, &tex_name, 100.);
            let combo_box = egui::ComboBox::new("tex_selector", "")
                .width(100.)
                .selected_text(egui::RichText::new(tex_name).color(tex_name_col));
            combo_box.show_ui(ui, |ui| {
                let mut texes = vec![];
                for style in &shared.armature.styles {
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
        shared.ui.styles_modal = true;
    } else if selected_tex != bone.tex {
        let mut anim_id = shared.ui.anim.selected;
        if !shared.ui.is_animating() {
            anim_id = usize::MAX;
        }
        shared.armature.set_bone_tex(
            bone.id,
            selected_tex,
            anim_id,
            shared.ui.anim.selected_frame,
        );
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

                let frame = shared.ui.anim.selected_frame;
                shared.save_edited_bone();
                shared.edit_bone(bone.id, $element, $float, anim_id, frame);
                *shared.saving.lock().unwrap() = shared::Saving::Autosaving;
            }
            if $label != "" {
                $ui.label($label);
            }
        };
    }

    // main macro to use for editable bone fields
    macro_rules! input {
        ($float:expr, $id:expr, $element:expr, $modifier:expr, $ui:expr, $label:expr) => {
            (edited, $float, _) = $ui.float_input($id.to_string(), shared, $float, $modifier, None);
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

    let has_ik = !bone.ik_disabled && shared.armature.bone_eff(bone.id) != JointEffector::None;
    let str_cant_edit = shared
        .loc("bone_panel.inverse_kinematics.cant_edit")
        .clone();

    ui.add_enabled_ui(!has_ik, |ui| {
        ui.horizontal(|ui| {
            label!(&shared.loc("bone_panel.position"), ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let pos_y = &AnimElement::PositionY;
                input!(bone.pos.y, "pos_y", pos_y, 1., ui, "Y");
                let pos_x = &AnimElement::PositionX;
                input!(bone.pos.x, "pos_x", pos_x, 1., ui, "X");
            })
        });

        ui.horizontal(|ui| {
            label!(&shared.loc("bone_panel.scale"), ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                input!(bone.scale.y, "scale_y", &AnimElement::ScaleY, 1., ui, "H");
                input!(bone.scale.x, "scale_x", &AnimElement::ScaleX, 1., ui, "W");
            });
        });
    })
    .response
    .on_disabled_hover_text(str_cant_edit);

    let not_end_ik = !has_ik || shared.armature.bone_eff(bone.id) == JointEffector::End;
    let str_cant_edit = shared
        .loc("bone_panel.inverse_kinematics.cant_edit")
        .clone();

    ui.add_enabled_ui(not_end_ik, |ui| {
        ui.horizontal(|ui| {
            label!(&shared.loc("bone_panel.rotation"), ui);
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
        label!(&shared.loc("bone_panel.zindex"), ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut zindex = bone.zindex as f32;
            input!(zindex, "zindex", &AnimElement::Zindex, 1., ui, "");
            bone.zindex = zindex as i32;
        });
    });

    let mut children = vec![];
    armature_window::get_all_children(&shared.armature.bones, &mut children, &bone);
    let parents = shared.armature.get_all_parents(bone.id);

    let section_spacing = 10.;

    if children.len() == 0
        && parents.len() == 0
        && bone.vertices.len() == 0
        && shared.armature.tex_of(bone.id) == None
    {
        ui.add_space(10.);
        let mut cache = egui_commonmark::CommonMarkCache::default();
        let loc = shared.loc("bone_panel.bone_empty").to_string();
        let str = utils::markdown(loc, shared.local_doc_url.to_string());
        egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
    }

    if children.len() > 0 || parents.len() > 0 {
        ui.add_space(section_spacing);
        inverse_kinematics(ui, shared, &bone);
        if !bone.ik_folded {
            ui.add_space(20.);
        }
    }

    if shared.armature.tex_of(bone.id) == None || bone.vertices.len() == 0 {
        return;
    }

    mesh_deformation(ui, shared, &bone);
}

pub fn inverse_kinematics(ui: &mut egui::Ui, shared: &mut Shared, bone: &Bone) {
    let str_heading = &shared.loc("bone_panel.inverse_kinematics.heading").clone();
    let str_desc = &shared.loc("bone_panel.inverse_kinematics.desc").clone();
    let frame = egui::Frame::new()
        .fill(shared.config.colors.dark_accent.into())
        .inner_margin(egui::Margin {
            bottom: 5,
            top: 5,
            left: 5,
            right: 5,
        });
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading.to_owned()).on_hover_text(str_desc);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fold_icon = if bone.ik_folded { "⏴" } else { "⏷" };
                let pointing_hand = egui::CursorIcon::PointingHand;
                if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                    shared.selected_bone_mut().unwrap().ik_folded =
                        !shared.selected_bone_mut().unwrap().ik_folded;
                }

                if shared.armature.bone_eff(bone.id) == JointEffector::None {
                    return;
                }

                let mut enabled = !bone.ik_disabled;
                let str_desc = &shared.loc("bone_panel.inverse_kinematics.enabled_desc");
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

                    if shared.armature.bone_eff(bone.id) == JointEffector::Start {
                        return;
                    }

                    // emable parents IK as well

                    let parents = shared.armature.get_all_parents(bone.id);
                    for parent in parents {
                        if shared.armature.bone_eff(parent.id) == JointEffector::None {
                            continue;
                        }

                        let bone = shared.armature.find_bone_mut(parent.id).unwrap();
                        bone.ik_disabled = !enabled;
                    }
                }
            })
        });
    });
    ui.add_space(2.5);

    if bone.ik_folded {
        return;
    }

    ui.horizontal(|ui| {
        //ui.label(&shared.loc("bone_panel.inverse_kinematics.effector"));
        ui.label(shared.loc("bone_panel.inverse_kinematics.family_index"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_selected = if bone.ik_family_id == -1 {
                shared.loc("none").to_string()
            } else {
                bone.ik_family_id.to_string()
            };
            egui::ComboBox::new("joint_eff", "")
                .selected_text(str_selected)
                .width(40.)
                .show_ui(ui, |ui| {
                    let mut selected = -1;

                    let mut ik_family_ids = vec![];
                    for bone in &shared.armature.bones {
                        if !ik_family_ids.contains(&bone.ik_family_id) && bone.ik_family_id != -1 {
                            ik_family_ids.push(bone.ik_family_id);
                        }
                    }

                    ui.selectable_value(&mut selected, -3, "None");
                    for id in &ik_family_ids {
                        ui.selectable_value(&mut selected, *id, id.to_string());
                    }
                    ui.selectable_value(&mut selected, -2, "New");

                    let bone = &mut shared.selected_bone_mut().unwrap();
                    if selected == -3 {
                        bone.ik_family_id = -1;
                    } else if selected == -2 {
                        let id = generate_id(ik_family_ids);
                        bone.ik_family_id = id;
                    } else if selected != -1 {
                        bone.ik_family_id = selected;
                    }
                })
                .response
                .on_hover_text(&shared.loc("bone_panel.inverse_kinematics.effector_desc"));
        });
    });

    if bone.ik_family_id == -1 {
        return;
    }

    let bones = &mut shared.armature.bones.iter();
    let ik_id = bone.ik_family_id;
    let root_id = bones.find(|b| b.ik_family_id == ik_id).unwrap().id;

    let go_to_root_str = shared.loc("bone_panel.inverse_kinematics.go_to-root");
    if root_id != bone.id {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(go_to_root_str).clicked() {
                    shared.ui.selected_bone_idx = bones.position(|b| b.id == root_id).unwrap();
                    shared.ui.selected_bone_ids = vec![];
                }
            });
        });
        return;
    }

    ui.horizontal(|ui| {
        ui.label(shared.loc("bone_panel.inverse_kinematics.mode_label"));
        let mode = &mut shared.selected_bone_mut().unwrap().ik_mode;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::new("ik_mode", "")
                .selected_text(mode.to_string())
                .width(40.)
                .show_ui(ui, |ui| {
                    ui.selectable_value(mode, InverseKinematicsMode::FABRIK, "FABRIK");
                    ui.selectable_value(mode, InverseKinematicsMode::Arc, "Arc");
                })
                .response
                .on_hover_text(str_desc);
        });
    });

    ui.horizontal(|ui| {
        let ik = "bone_panel.inverse_kinematics.";
        ui.label(&shared.loc(&(ik.to_owned() + "constraint")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_none = &shared.loc("none").clone();
            let str_clockwise = shared.loc(&(ik.to_owned() + "Clockwise")).clone() + "  ⟳";
            let str_ccw = shared.loc(&(ik.to_owned() + "CounterClockwise")).clone() + "  ⟲";
            let str_desc = &shared.loc(&(ik.to_owned() + "constraint_desc")).clone();
            let selected = match bone.ik_constraint {
                JointConstraint::Clockwise => str_clockwise.clone(),
                JointConstraint::CounterClockwise => str_ccw.clone(),
                _ => str_none.clone(),
            };

            egui::ComboBox::new("joint_constraint", "")
                .selected_text(selected)
                .width(40.)
                .show_ui(ui, |ui| {
                    let mut ik = shared.selected_bone_mut().unwrap().ik_constraint.clone();
                    ui.selectable_value(&mut ik, JointConstraint::None, str_none);
                    ui.selectable_value(&mut ik, JointConstraint::Clockwise, str_clockwise);
                    ui.selectable_value(&mut ik, JointConstraint::CounterClockwise, str_ccw);
                    let sel = shared.ui.anim.selected;
                    let frame = shared.ui.anim.selected_frame;
                    let constraint = AnimElement::IkConstraint;
                    shared.edit_bone(bone.id, &constraint, (ik as usize) as f32, sel, frame);
                })
                .response
                .on_hover_text(str_desc);
        });
    });

    let target_buttons_width = 60.;

    ui.horizontal(|ui| {
        ui.label(&shared.loc("bone_panel.inverse_kinematics.target"));

        let bone_id = bone.ik_target_id;
        if let Some(target) = shared.armature.bones.iter().find(|b| b.id == bone_id) {
            let width = ui.available_width();
            let tr_name = utils::trunc_str(ui, &target.name.clone(), width - target_buttons_width);
            if ui.selectable_label(false, tr_name).clicked() {
                let bones = &mut shared.armature.bones;
                let ik_id = bone.ik_target_id;
                shared.ui.selected_bone_idx = bones.iter().position(|b| b.id == ik_id).unwrap();
            };
        } else {
            ui.label(shared.loc("none"));
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
                let button = ui.skf_button("🗑");
                if button.on_hover_text(str_remove_target).clicked() {
                    shared.selected_bone_mut().unwrap().ik_target_id = -1;
                }
            });

            if ui.skf_button("⌖").on_hover_text(str_set_target).clicked() {
                shared.ui.setting_ik_target = true;
            }
        });
    });
}

pub fn mesh_deformation(ui: &mut egui::Ui, shared: &mut Shared, bone: &Bone) {
    let str_heading = &shared.loc("bone_panel.mesh_deformation.heading").clone();
    let str_desc = &shared.loc("bone_panel.mesh_deformation.desc").clone();

    let frame = egui::Frame::new()
        .fill(shared.config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading.to_owned()).on_hover_text(str_desc);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fold_icon = if bone.meshdef_folded { "⏴" } else { "⏷" };
                let pointing_hand = egui::CursorIcon::PointingHand;
                if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                    shared.selected_bone_mut().unwrap().meshdef_folded =
                        !shared.selected_bone_mut().unwrap().meshdef_folded;
                }
            })
        });
    });
    ui.add_space(2.5);

    if bone.meshdef_folded {
        return;
    }

    // check if this bone is a weight
    let parents = shared.armature.get_all_parents(bone.id);
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
        let str = &shared.loc("bone_panel.mesh_deformation.go_to_mesh").clone();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.skf_button(str).clicked() {
                    let bones = &shared.armature.bones;
                    let idx = bones.iter().position(|b| b.id == mesh_parent_id).unwrap();
                    shared.ui.select_bone(idx);
                }
            });
        });

        return;
    }

    if shared.armature.tex_of(bone.id) == None {
        return;
    }

    let str_edit = &shared.loc("bone_panel.mesh_deformation.edit").clone();
    let str_finish_edit = &shared
        .loc("bone_panel.mesh_deformation.finish_edit")
        .clone();
    let mut mesh_label = str_edit;
    if shared.ui.showing_mesh {
        mesh_label = str_finish_edit;
    }

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button(&mesh_label).clicked() {
                shared.ui.showing_mesh = !shared.ui.showing_mesh;
            }
        });
    });

    ui.add_enabled_ui(shared.ui.showing_mesh, |ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let tex_size = shared.armature.tex_of(bone.id).unwrap().size.clone();

                let str_reset = &shared.loc("bone_panel.mesh_deformation.reset");
                //let str_reset_desc = &shared.loc("bone_panel.mesh_deformation.reset_desc");
                let can_reset = !shared.ui.setting_bind_verts;
                if ui
                    .add_enabled(can_reset, egui::Button::new(str_reset))
                    .clicked()
                {
                    let (verts, indices) = renderer::create_tex_rect(&tex_size);
                    let bone = shared.selected_bone_mut().unwrap();
                    bone.vertices = verts;
                    bone.indices = indices;
                    bone.binds = vec![];
                    bone.verts_edited = false;
                    shared.ui.selected_bind = -1;
                }

                let str_center = &shared.loc("bone_panel.mesh_deformation.center");
                let str_center_desc = &shared.loc("bone_panel.mesh_deformation.center_desc");
                let button = ui.skf_button(str_center);
                if button.on_hover_text(str_center_desc).clicked() {
                    center_verts(&mut shared.selected_bone_mut().unwrap().vertices);
                }

                let trace_str = &shared.loc("bone_panel.mesh_deformation.trace");
                if ui.skf_button(trace_str).clicked() {
                    let tex = &shared.armature.tex_of(bone.id).unwrap();
                    let tex_data = &shared.armature.tex_data;
                    let data = tex_data.iter().find(|d| tex.data_id == d.id).unwrap();
                    let (verts, indices) = renderer::trace_mesh(&data.image);
                    let bone = &mut shared.selected_bone_mut().unwrap();
                    bone.vertices = verts;
                    bone.indices = indices;
                    bone.binds = vec![];
                    bone.verts_edited = true;
                    shared.ui.selected_bind = -1;
                }
            });
        });
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.label(shared.loc("bone_panel.mesh_deformation.binds_label"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let headline = if shared.ui.selected_bind == -1 {
                shared.loc("none").to_string()
            } else {
                shared.ui.selected_bind.to_string()
            };
            let combo_box = egui::ComboBox::new("bone_weights", "").selected_text(headline);
            combo_box.show_ui(ui, |ui| {
                let mut selected_value: i32 = -1;
                for b in 0..bone.binds.len() {
                    ui.selectable_value(&mut selected_value, b as i32, b.to_string());
                }
                ui.selectable_value(&mut selected_value, -2, shared.loc("new_option"));

                if selected_value == -2 {
                    let binds = &mut shared.selected_bone_mut().unwrap().binds;
                    binds.push(BoneBind {
                        bone_id: -1,
                        ..Default::default()
                    });
                    shared.ui.selected_bind = binds.len() as i32 - 1;
                } else if selected_value != -1 {
                    shared.ui.selected_bind = selected_value;
                }
            });
        });
    });

    if shared.ui.selected_bind == -1 {
        return;
    }

    let binds = shared.selected_bone().unwrap().binds.clone();
    ui.horizontal(|ui| {
        let bone_id = binds[shared.ui.selected_bind as usize].bone_id;
        let mut bone_name = shared.loc("none").to_string();
        if let Some(bone) = shared.armature.bones.iter().find(|bone| bone.id == bone_id) {
            bone_name = bone.name.clone();
        }
        ui.label(shared.loc("bone_panel.mesh_deformation.bone_label") + &bone_name);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_set_bone = if shared.ui.setting_bind_bone {
                shared.loc("bone_panel.mesh_deformation.finish")
            } else {
                shared.loc("bone_panel.mesh_deformation.bone_set")
            };
            if ui.skf_button(&str_set_bone).clicked() {
                shared.ui.setting_bind_bone = !shared.ui.setting_bind_bone;
            }
        });
    });
    let selected = shared.ui.selected_bind as usize;

    if binds[selected].bone_id == -1 {
        return;
    }

    let vert_id_len = shared.selected_bone().unwrap().binds[selected].verts.len();
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_set_verts = if shared.ui.setting_bind_verts {
                shared.loc("bone_panel.mesh_deformation.finish")
            } else {
                shared.loc("bone_panel.mesh_deformation.bind_verts")
            };
            if ui.skf_button(&str_set_verts).clicked() {
                shared.ui.setting_bind_verts = !shared.ui.setting_bind_verts;
                if shared.was_editing_path {
                    shared.selected_bone_mut().unwrap().binds[selected].is_path = true;
                    shared.was_editing_path = false;
                } else {
                    shared.was_editing_path = binds[selected].is_path;
                    shared.selected_bone_mut().unwrap().binds[selected].is_path = false;
                }
            }
        });
    });

    ui.horizontal(|ui| {
        if vert_id_len > 0 {
            ui.label(shared.loc("bone_panel.mesh_deformation.weights_label"));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let binding_verts = shared.ui.setting_bind_verts;
            let bind = &mut shared.selected_bone_mut().unwrap().binds[selected];
            if binding_verts {
                ui.add_enabled_ui(false, |ui| {
                    ui.checkbox(&mut shared.was_editing_path, "".into_atoms());
                });
            } else {
                ui.checkbox(&mut bind.is_path, "".into_atoms());
            }

            ui.label(shared.loc("bone_panel.mesh_deformation.pathing_label"))
                .on_hover_text(shared.loc("bone_panel.mesh_deformation.pathing_desc"));
        });
    });

    let selected = shared.ui.selected_bind;
    let binds = &mut shared.selected_bone_mut().unwrap().binds[selected as usize];
    if binds.verts.len() == 0 {
        ui.label(shared.loc("bone_panel.mesh_deformation.no_bound_verts"));
    } else {
        for w in 0..binds.verts.len() {
            ui.horizontal(|ui| {
                let str_label = binds.verts[w].id.to_string() + ":";
                ui.label(str_label);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut binds.verts[w].weight, (0.)..=1.))
                    });
                });
            });
        }
    }
}

pub fn center_verts(verts: &mut Vec<Vertex>) {
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
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_file_dialog(file_name: Arc<Mutex<String>>, file_contents: Arc<Mutex<Vec<u8>>>) {
    thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg", "tif"])
            .pick_file();
        if task == None {
            return;
        }
        let file_str = task.as_ref().unwrap().as_path().to_str();
        *file_name.lock().unwrap() = file_str.unwrap().to_string();
        *file_contents.lock().unwrap() =
            fs::read(task.unwrap().as_path().to_str().unwrap()).unwrap();
    });
}
