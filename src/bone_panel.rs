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
    if shared.ui.has_state(UiState::DraggingBone) {
        ui.disable();
        return;
    }

    ui.horizontal(|ui| {
        ui.heading(&shared.loc("bone_panel.heading"));

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
                let str = &shared.loc("polar.delete_bone").clone();
                shared.ui.context_menu.id = shared.selected_bone().unwrap().id;
                shared.ui.open_polar_modal(PolarId::DeleteBone, &str);
            }
        });
    });

    ui.separator();
    ui.add_space(3.);

    ui.horizontal(|ui| {
        ui.label(&shared.loc("bone_panel.name"));
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

    ui.horizontal(|ui| {
        ui.label(&shared.loc("bone_panel.style"));

        let name = if let Some(set) = shared.armature.get_current_set(bone.id) {
            set.name.clone()
        } else {
            "None".to_string()
        };

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::new("bone_style_drop", "")
                .selected_text(name.clone())
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show_ui(ui, |ui| {
                    let mut selected_value = -1;
                    for s in 0..shared.armature.styles.len() {
                        let label = ui.selectable_value(
                            &mut selected_value,
                            s as i32,
                            shared.armature.styles[s].name.to_string(),
                        );

                        if bone.style_ids.contains(&(s as i32)) {
                            ui.painter().text(
                                label.rect.right_center(),
                                egui::Align2::RIGHT_CENTER,
                                "✅",
                                egui::FontId::default(),
                                shared.config.colors.text.into(),
                            );
                        }
                    }
                    ui.selectable_value(
                        &mut selected_value,
                        -2,
                        &shared.loc("bone_panel.texture_set_setup"),
                    );

                    if selected_value == -2 {
                        shared.open_style_modal();
                        ui.close();
                    } else if selected_value != -1 {
                        let styles = &mut shared.armature.find_bone_mut(bone.id).unwrap().style_ids;
                        if styles.contains(&selected_value) {
                            let idx = styles
                                .iter()
                                .position(|style| *style == selected_value)
                                .unwrap();
                            styles.remove(idx);
                        } else {
                            styles.push(selected_value);
                        }
                        shared.armature.set_bone_tex(
                            bone.id,
                            bone.tex_idx as usize,
                            usize::MAX,
                            -1,
                        );
                    }
                });
        });
    });

    let tex = shared.armature.get_current_tex(bone.id);
    let set = shared.armature.get_current_set(bone.id);

    let tex_name_col = if tex != None {
        shared.config.colors.text
    } else {
        shared.config.colors.light_accent + Color::new(60, 60, 60, 0)
    };

    let mut selected_tex = bone.tex_idx;
    let tex_name = if tex != None {
        &tex.unwrap().name
    } else {
        &"None".to_string()
    };
    let str_idx = bone.tex_idx.to_string() + ") ";
    ui.add_enabled_ui(set != None, |ui| {
        ui.horizontal(|ui| {
            ui.label(&shared.loc("bone_panel.texture_index"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::new("tex_selector", "")
                    .selected_text(egui::RichText::new(str_idx + tex_name).color(tex_name_col))
                    .show_ui(ui, |ui| {
                        let set = &shared.armature.get_current_set(bone.id).unwrap();
                        for t in 0..set.textures.len() {
                            let str_idx = t.to_string() + ") ";
                            ui.selectable_value(
                                &mut selected_tex,
                                t as i32,
                                str_idx + &set.textures[t].name.clone(),
                            );
                        }
                    })
                    .response;
            });
        });
    });

    if set != None && selected_tex != bone.tex_idx {
        let mut anim_id = shared.ui.anim.selected;
        if !shared.ui.is_animating() {
            anim_id = usize::MAX;
        }
        shared.armature.set_bone_tex(
            bone.id,
            selected_tex as usize,
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

                shared.save_edited_bone();
                shared.armature.edit_bone(
                    bone.id,
                    $element,
                    $float,
                    anim_id,
                    shared.ui.anim.selected_frame,
                );
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

    if children.len() > 0 || parents.len() > 0 {
        ui.add_space(section_spacing);
        inverse_kinematics(ui, shared, &bone);
        ui.add_space(20.);
    }

    // disabled: mesh deformation (not ready either)
    if !shared.config.meshdef {
        return;
    }

    if bone.vertices.len() == 0 {
        return;
    }

    mesh_deformation(ui, shared, &bone);
}

pub fn inverse_kinematics(ui: &mut egui::Ui, shared: &mut Shared, bone: &Bone) {
    let str_heading = &shared.loc("bone_panel.inverse_kinematics.heading").clone();
    let str_desc = &shared.loc("bone_panel.inverse_kinematics.desc").clone();
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

    ui.separator();

    if bone.ik_folded {
        return;
    }

    ui.horizontal(|ui| {
        //ui.label(&shared.loc("bone_panel.inverse_kinematics.effector"));
        ui.label("Family Index: ");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_selected = if bone.ik_family_id == -1 {
                "None".to_string()
            } else {
                bone.ik_family_id.to_string()
            };
            egui::ComboBox::new("joint_eff", "")
                .selected_text(str_selected)
                .width(40.)
                .show_ui(ui, |ui| {
                    let mut selected = -1;

                    let mut ik_family_ids: Vec<i32> = shared
                        .armature
                        .bones
                        .iter()
                        .map(|bone| bone.ik_family_id)
                        .filter(|id| *id != -1)
                        .collect();
                    ik_family_ids.dedup();

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

    let root_joint_id = shared
        .armature
        .bones
        .iter()
        .find(|other| other.ik_family_id == bone.ik_family_id)
        .unwrap()
        .id;

    if root_joint_id != bone.id {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Go to root joint").clicked() {
                    shared.ui.selected_bone_idx = shared
                        .armature
                        .bones
                        .iter()
                        .position(|other| other.id == root_joint_id)
                        .unwrap();
                    shared.ui.selected_bone_ids = vec![];
                }
            });
        });
        return;
    }

    ui.horizontal(|ui| {
        let ik = "bone_panel.inverse_kinematics.";
        ui.label(&shared.loc(&(ik.to_owned() + "constraint")));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_none = &shared.loc(&(ik.to_owned() + "None")).clone();
            let str_clockwise = shared.loc(&(ik.to_owned() + "Clockwise")).clone() + "  ⟳";
            let str_ccw = shared.loc(&(ik.to_owned() + "CounterClockwise")).clone() + "  ⟲";
            let str_desc = &shared.loc(&(ik.to_owned() + "constraint_desc")).clone();
            let selected = match bone.constraint {
                JointConstraint::Clockwise => str_clockwise.clone(),
                JointConstraint::CounterClockwise => str_ccw.clone(),
                JointConstraint::None => str_none.clone(),
            };

            egui::ComboBox::new("joint_constraint", "")
                .selected_text(selected)
                .width(40.)
                .show_ui(ui, |ui| {
                    let constraint = &mut shared.selected_bone_mut().unwrap().constraint;
                    ui.selectable_value(constraint, JointConstraint::None, str_none);
                    ui.selectable_value(constraint, JointConstraint::Clockwise, str_clockwise);
                    ui.selectable_value(constraint, JointConstraint::CounterClockwise, str_ccw);
                })
                .response
                .on_hover_text(str_desc);
        });
    });

    let mut is_target_newline = false;
    let target_buttons_width = 40.;

    ui.horizontal(|ui| {
        ui.label(&shared.loc("bone_panel.inverse_kinematics.target"));

        if let Some(target) = shared.armature.find_bone(bone.ik_target_id) {
            if ui.selectable_label(false, target.name.clone()).clicked() {
                shared.ui.selected_bone_idx =
                    shared.armature.find_bone_idx(bone.ik_target_id).unwrap();
            };
        } else {
            ui.label("None");
        }

        is_target_newline = ui.available_width() < target_buttons_width;

        if is_target_newline {
            return;
        }

        target_buttons(ui, shared, bone);
    });

    if is_target_newline {
        ui.horizontal(|ui| {
            target_buttons(ui, shared, bone);
        });
    }
}

pub fn target_buttons(ui: &mut egui::Ui, shared: &mut Shared, bone: &Bone) {
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
}

pub fn mesh_deformation(ui: &mut egui::Ui, shared: &mut Shared, bone: &Bone) {
    let str_heading = &shared.loc("bone_panel.mesh_deformation.heading").clone();
    let str_desc = &shared.loc("bone_panel.mesh_deformation.desc").clone();
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

    if bone.meshdef_folded || shared.armature.get_current_tex(bone.id) == None {
        return;
    }

    let str_edit = &shared.loc("bone_panel.mesh_deformation.edit").clone();
    let str_finish_edit = &shared
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
            let tex_size = shared
                .armature
                .get_current_tex(bone.id)
                .unwrap()
                .size
                .clone();
            //let str_center = &shared.loc("bone_panel.mesh_deformation.center");
            //let str_center_desc = &shared.loc("bone_panel.mesh_deformation.center_desc");
            //let button = ui.skf_button(str_center);
            //if button.on_hover_text(str_center_desc).clicked() {
            //    center_verts(&mut shared.selected_bone_mut().unwrap().vertices, &tex_size);
            //}
            let str_reset = &shared.loc("bone_panel.mesh_deformation.reset");
            let str_reset_desc = &shared.loc("bone_panel.mesh_deformation.reset_desc");
            let button = ui.skf_button(str_reset);
            if button.on_hover_text(str_reset_desc).clicked() {
                let (verts, indices) = renderer::create_tex_rect(&tex_size);
                shared.selected_bone_mut().unwrap().vertices = verts;
                shared.selected_bone_mut().unwrap().indices = indices;
            }

            if ui.skf_button("Trace").clicked() {
                let (verts, indices) = renderer::trace_mesh(
                    &shared.armature.get_current_set(bone.id).unwrap().textures
                        [bone.tex_idx as usize]
                        .image,
                );
                shared.selected_bone_mut().unwrap().vertices = verts;
                shared.selected_bone_mut().unwrap().indices = indices;
            }
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
