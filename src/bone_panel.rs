//! UI Bone window.

use crate::*;
use egui::IntoAtoms;
use ui::EguiUi;
type AE = AnimElement;

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use crate::file_reader::*;
    pub use std::path::PathBuf;
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
type E = AnimElement;

pub fn draw(
    mut bone: Bone,
    ui: &mut egui::Ui,
    selections: &mut SelectionState,
    shared_ui: &mut crate::Ui,
    armature: &mut Armature,
    config: &Config,
    events: &mut EventState,
    input: &InputStates,
    edit_mode: &EditMode,
) {
    let sel = &selections.clone();
    if armature.bones.len() == 0 || armature.sel_bone(sel) == None {
        ui.disable();
        return;
    }

    ui.horizontal(|ui| {
        ui.heading(&shared_ui.loc("bone_panel.heading"));
        let hand = egui::CursorIcon::PointingHand;

        let icon_widths = 90.;
        //ui.add_space(ui.available_width() - icon_widths);
        //let rect =
        //    egui::Rect::from_min_size(ui.cursor().left_top() + [0., 3.].into(), [13., 17.].into());
        //let response: egui::Response = ui
        //    .allocate_rect(rect, egui::Sense::click())
        //    .on_hover_cursor(hand);
        //egui::Image::new(shared_ui.kite_img.as_ref().unwrap()).paint_at(ui, rect);
        //if response.clicked() {
        //    egui::color_picker::color_picker_color32(
        //        ui,
        //        &mut bone.group_color.egui_rgba(),
        //        egui::color_picker::Alpha::BlendOrAdditive,
        //    );
        //}
        ui.add_space(ui.available_width() - icon_widths);

        let og_col: [u8; 4] = [
            bone.group_color.r,
            bone.group_color.g,
            bone.group_color.b,
            bone.group_color.a,
        ];
        let mut col = og_col.clone();
        let tooltip = shared_ui.loc("bone_panel.group_color_desc");
        let group_color_button = ui
            .color_edit_button_srgba_premultiplied(&mut col)
            .on_hover_text(tooltip);

        // focus on the first element of bone panel, upon selecting a new bone
        if shared_ui.prev_selected_bone_idx != selections.bone_idx {
            group_color_button.request_focus();
            shared_ui.prev_selected_bone_idx = selections.bone_idx;
        }

        if col != og_col {
            events.edit_bone(bone.id, &E::GroupColorR, col[0] as f32, "", usize::MAX, -1);
            events.edit_bone(bone.id, &E::GroupColorG, col[1] as f32, "", usize::MAX, -1);
            events.edit_bone(bone.id, &E::GroupColorB, col[2] as f32, "", usize::MAX, -1);
            events.edit_bone(bone.id, &E::GroupColorA, col[3] as f32, "", usize::MAX, -1);
        }

        let mut col = config.colors.text;
        if !bone.locked {
            col -= Color::new(60, 60, 60, 0);
        }

        let offset = ui.cursor().min + [0., 3.].into();
        let rect = egui::Rect::from_min_size(offset, [15., 15.].into());
        let img = shared_ui.lock_img.as_ref().unwrap();
        let response: egui::Response = ui
            .allocate_rect(rect, egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(shared_ui.loc("locked_desc"));
        if response.hovered() || response.has_focus() {
            col += Color::new(60, 60, 60, 0);
        }
        egui::Image::new(img).tint(col).paint_at(ui, rect);
        if response.clicked() {
            let locked_f32 = if bone.locked { 0. } else { 1. };
            let locked = &AnimElement::Locked;
            events.edit_bone(bone.id, locked, locked_f32, "", usize::MAX, -1);
        }

        let mut col = config.colors.text;
        col -= Color::new(60, 60, 60, 0);
        let text = egui::RichText::new("🗑").size(15.).color(col);
        if ui.label(text).on_hover_cursor(hand).clicked() {
            let str = shared_ui.loc("polar.delete_bone").clone().to_string();
            let context_id = format!("b_{}", sel.bone_idx.to_string());
            shared_ui.context_menu.id = context_id;
            shared_ui.context_menu.keep = true;
            events.open_polar_modal(PolarId::DeleteBone, str);
        }
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
                let bone_ids = selections.only_root_bones(&armature.bones);
                edit_bones(&bone_ids, $element, $float, "", anim_id, frame, events);
                *shared_ui.saving.lock().unwrap() = shared::Saving::Autosaving;
            }
        };
    }

    // main macro to use for editable bone fields
    macro_rules! input {
        ($float:expr, $id:expr, $element:expr, $modifier:expr, $drag_mod: expr, $ui:expr, $label:expr) => {
            if $label != "" {
                $ui.label($label);
            }
            let options = Some(crate::ui::TextInputOptions {
                size: Vec2::new(40., 20.),
                drag_modifier: $drag_mod,
                ..Default::default()
            });
            let id = $id.to_string();
            (edited, $float, _) = $ui.float_input(id, shared_ui, $float, $modifier, options);
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

    let input_widths = 120.;
    let mut children = vec![];
    armature_window::get_all_children(&armature.bones, &mut children, &bone);
    let parents = armature.get_all_parents(false, bone.id);

    let has_ik = bone.ik_family_id != -1
        && !bone.ik_disabled
        && armature.bone_eff(bone.id) != JointEffector::Start;
    let str_cant_edit = shared_ui
        .loc("bone_panel.inverse_kinematics.cant_edit")
        .clone();

    type AE = AnimElement;
    ui.add_enabled_ui(!has_ik, |ui| {
        ui.horizontal(|ui| {
            label!(&shared_ui.loc("bone_panel.position"), ui);
            ui.add_space(ui.available_width() - input_widths);
            input!(bone.pos.x, "pos_x", AE::PositionX, 1., 1., ui, "X");
            input!(bone.pos.y, "pos_y", AE::PositionY, 1., 1., ui, "Y");
        });

        ui.horizontal(|ui| {
            label!(&shared_ui.loc("bone_panel.scale"), ui);
            ui.add_space(ui.available_width() - input_widths - 5.);
            type AE = AnimElement;
            input!(bone.scale.x, "scale_x", AE::ScaleX, 1., 0.05, ui, "W");
            input!(bone.scale.y, "scale_y", AE::ScaleY, 1., 0.05, ui, "H");
        });

        ui.horizontal(|ui| {
            label!(&shared_ui.loc("bone_panel.rotation"), ui);
            ui.add_space(ui.available_width() - 40.);
            let rot_el = AnimElement::Rotation;
            let deg_mod = 180. / std::f32::consts::PI;
            input!(bone.rot, "rot", rot_el, deg_mod, -0.25, ui, "");
        });
    })
    .response
    .on_disabled_hover_text(str_cant_edit);

    ui.add_space(20.);

    // show 'IK root bone' button if this is a target bone
    let bones = &mut armature.bones.iter();
    let is_target_of = bones.position(|b| b.ik_family_id != -1 && b.ik_target_id == bone.id);
    if is_target_of != None {
        let target_str = shared_ui.loc("bone_panel.target_bone");
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
        //let mut cache = egui_commonmark::CommonMarkCache::default();
        //let loc = shared_ui.loc("bone_panel.bone_empty").to_string();
        //let str = utils::markdown(loc, shared_ui.local_doc_url.to_string());
        //egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
    }

    // show mesh deformation, if all selected bones are eligible
    let mut all_can_mesh = true;
    for id in &selections.bone_ids {
        let bone = armature.bones.iter().find(|b| b.id == *id).unwrap().clone();
        if bone.vertices.len() == 0 || armature.tex_of(bone.id) == None {
            all_can_mesh = false;
        }
    }
    if all_can_mesh {
        let mut is_hovering = mesh_deformation(
            ui, &bone, shared_ui, events, config, selections, armature, edit_mode,
        );
        // show vertex position and UV inputs, if one is selected
        if selections.vert_ids.len() > 0 {
            ui.add_space(10.);
            ui.separator();
            is_hovering =
                selected_verts_inputs(ui, shared_ui, selections, &bone, events) || is_hovering;
        }
        // if no vertex labels were hovered from both funcs above, set hovered vert to none
        if !is_hovering && selections.hovering_vert_id != -1 {
            events.set_hovering_id(-1);
        }
        ui.add_space(20.);
    }

    if children.len() > 0 || parents.len() > 0 {
        inverse_kinematics(ui, &bone, selections, shared_ui, config, armature, events);
        if !bone.ik_folded {
            ui.add_space(20.);
        }
    }

    // physics is not part of v0.4
    //#[cfg(not(debug_assertions))]
    //return;

    physics(ui, &bone, selections, shared_ui, config, armature, events);

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
        .inner_margin(egui::Margin::same(5));

    let bones = &armature.bones;
    let ik_id = bone.ik_family_id;
    let root_id = bones.iter().find(|b| b.ik_family_id == ik_id).unwrap().id;

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading).on_hover_text(str_desc);
            let color = config.colors.inverse_kinematics;
            let pos = egui::Pos2::new(ui.cursor().left(), ui.cursor().top() + 4.);
            let rect = egui::Rect::from_min_size(pos, [13., 10.].into());
            let img = shared_ui.ik_img.as_ref().unwrap();
            egui::Image::new(img).tint(color).paint_at(ui, rect);

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

                //let mut enabled = !bone.ik_disabled;
                //let str_desc = &shared_ui.loc("bone_panel.inverse_kinematics.enabled_desc");
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
                        type A = AnimElement;
                        let mut id = selected;
                        if selected == -2 {
                            id = generate_id(ik_family_ids);
                        } else if selected == -3 {
                            id = -1;
                        } else {
                            // reset Y pos if this is a child IK bone
                            events.edit_bone(bone.id, &A::PositionY, 0., "", usize::MAX, -1);
                        }
                        events.edit_bone(bone.id, &A::IkFamilyId, id as f32, "", usize::MAX, -1);
                    }
                })
                .response
                .on_hover_text(&shared_ui.loc("bone_panel.inverse_kinematics.effector_desc"));
        });
    });

    if bone.ik_family_id == -1 {
        return;
    }

    let go_to_root_str = shared_ui.loc("bone_panel.inverse_kinematics.go_to_root");
    if root_id != bone.id {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("bone_panel.inverse_kinematics.distance"))
                .on_hover_text(shared_ui.loc("bone_panel.inverse_kinematics.distance_desc"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let id = "ik_distance".to_string();
                let (edited, value, _) = ui.float_input(id, shared_ui, bone.pos.x, 1., None);
                if edited {
                    events.edit_bone(bone.id, &AE::PositionX, value, "", usize::MAX, -1);
                    events.edit_bone(bone.id, &AE::PositionY, 0., "", usize::MAX, -1);
                }
            });
        });

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
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::new("ik_mode", "")
                .selected_text(bone.ik_mode.to_string())
                .width(40.)
                .show_ui(ui, |ui| {
                    let mut selected_mode = -1;
                    ui.selectable_value(&mut selected_mode, 0, "FABRIK");
                    ui.selectable_value(&mut selected_mode, 1, "Arc");
                    #[rustfmt::skip]
                    if selected_mode != -1 {
                        let mode = &InverseKinematicsMode::from_repr(selected_mode).unwrap().to_string();
                        events.edit_bone(bone.id, &AnimElement::IkMode, f32::MAX, mode, selections.anim, selections.anim_frame);
                    };
                })
                .response
                .on_hover_text(str_desc);
        });
    });

    ui.horizontal(|ui| {
        let ik = "bone_panel.inverse_kinematics.";
        ui.label(&shared_ui.loc(&format!("{}constraint", ik)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_none = &shared_ui.loc("none").clone();
            let str_clockwise = shared_ui.loc(&format!("{}Clockwise", ik)) + "  ⟳";
            let str_ccw = shared_ui.loc(&format!("{}CounterClockwise", ik)) + "  ⟲";
            let str_desc = &shared_ui.loc(&format!("{}constraint_desc", ik));
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
                        #[rustfmt::skip]
                        events.edit_bone(bone.id, &AnimElement::IkConstraint, f32::MAX, &ik.to_string(), selections.anim, selections.anim_frame);
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
    edit_mode: &EditMode,
) -> bool {
    let str_heading = &shared_ui.loc("bone_panel.mesh_deformation.heading").clone();
    let str_desc = &shared_ui.loc("bone_panel.mesh_deformation.desc").clone();

    let sel = selections.clone();

    let frame = egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading).on_hover_text(str_desc);
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
        return false;
    }

    // check if this bone is a weight
    let parents = armature.get_all_parents(false, bone.id);
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
    }

    if armature.tex_of(bone.id) == None {
        return false;
    }

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // "edit vertices" / "finish editing" label
            let str_edit = &shared_ui.loc("bone_panel.mesh_deformation.edit").clone();
            let str_finish_edit = &shared_ui
                .loc("bone_panel.mesh_deformation.finish_edit")
                .clone();
            let mut mesh_label = str_edit;
            if edit_mode.showing_mesh {
                mesh_label = str_finish_edit;
            }

            // formatted label with `Edit Vertices [toggle_key]`
            let label = mesh_label;
            let mut str = egui::text::LayoutJob::default();
            ui::job_text(&format!("{}  ", label), None, &mut str);
            let mut col = config.colors.text;
            col -= Color::new(50, 50, 50, 0);
            let key = config.keys.toggle_edit_vertices.display();
            ui::job_text(&key, Some(col.into()), &mut str);

            if ui.skf_button(str).clicked() {
                events.toggle_showing_mesh(if edit_mode.showing_mesh { 0 } else { 1 });
            }
        });
    });

    ui.add_enabled_ui(edit_mode.showing_mesh, |ui| {
        ui.horizontal(|ui| {
            let button_widths = 149.;
            ui.add_space(ui.available_width() - button_widths);

            // tracing button
            let desc_str;
            let str = if !shared_ui.tracing {
                desc_str = "bone_panel.mesh_deformation.trace_desc";
                "bone_panel.mesh_deformation.trace"
            } else {
                desc_str = "bone_panel.mesh_deformation.trace_finish_desc";
                "bone_panel.mesh_deformation.finish"
            };
            let trace_str = &shared_ui.loc(str);
            let button = ui
                .sized_skf_button([45., 20.], trace_str)
                .on_hover_text(shared_ui.loc(desc_str));
            if button.clicked() {
                shared_ui.tracing = !shared_ui.tracing;
                if shared_ui.tracing {
                    events.save_bone(armature.bones.iter().position(|b| b.id == bone.id).unwrap());
                    events.trace_bone_verts();
                }
            }

            // center button
            let str_center = &shared_ui.loc("bone_panel.mesh_deformation.center");
            let str_center_desc = &shared_ui.loc("bone_panel.mesh_deformation.center_desc");
            let button = ui.sized_skf_button([40., 20.], str_center);
            if button.on_hover_text(str_center_desc).clicked() {
                events.center_bone_verts();
            }

            // reset button
            let str_reset = &shared_ui.loc("bone_panel.mesh_deformation.reset");
            let str_reset_desc = &shared_ui.loc("bone_panel.mesh_deformation.reset_desc");
            let can_reset = selections.bind == -1;
            ui.add_enabled_ui(can_reset, |ui| {
                let button = ui
                    .add_sized([40., 20.], egui::Button::new(str_reset))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(str_reset_desc);
                if button.clicked() {
                    events.reset_vertices();
                }
            });
        });
    });

    if shared_ui.tracing {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let gap = shared_ui.tracing_gap;
                let (edited, value, _) =
                    ui.float_input("gap".to_string(), shared_ui, gap, 1., None);
                if edited {
                    shared_ui.tracing_gap = value.max(1.);
                    events.trace_bone_verts();
                }
                ui.label(shared_ui.loc("bone_panel.mesh_deformation.gap"))
                    .on_hover_text(shared_ui.loc("bone_panel.mesh_deformation.gap_desc"));
            });
        });
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let padding = shared_ui.tracing_padding;
                let (edited, value, _) =
                    ui.float_input("padding".to_string(), shared_ui, padding, 1., None);
                if edited {
                    shared_ui.tracing_padding = value;
                    events.trace_bone_verts();
                }
                ui.label(shared_ui.loc("bone_panel.mesh_deformation.padding"))
                    .on_hover_text(shared_ui.loc("bone_panel.mesh_deformation.padding_desc"));
            });
        });
    }

    ui.separator();

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("bone_panel.mesh_deformation.binds_label"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let headline = if selections.bind == -1 {
                shared_ui.loc("none").to_string()
            } else {
                selections.bind.to_string()
            };
            let id = format!("bone_weights{}", bone.binds.len().to_string());
            let combo_box = egui::ComboBox::new(id, "")
                .selected_text(headline)
                .height(1000.);
            combo_box.show_ui(ui, |ui| {
                let mut selected_value: i32 = -1;
                ui.selectable_value(&mut selected_value, -3, shared_ui.loc("none_option"));
                for b in 0..bone.binds.len() {
                    ui.selectable_value(&mut selected_value, b as i32, b.to_string());
                }
                ui.selectable_value(&mut selected_value, -2, shared_ui.loc("new_option"));
                events.select_bind(selected_value);
            });
        });
    });

    if selections.bind == -1 {
        return false;
    }

    let binds = armature.sel_bone(&sel).unwrap().binds.clone();
    if binds.len() == 0 || selections.bind as usize > binds.len() - 1 {
        selections.bind = -1;
        return false;
    }

    ui.horizontal(|ui| {
        let bone_id = binds[selections.bind as usize].bone_id;
        let mut bone_name = shared_ui.loc("none").to_string();
        if let Some(bone) = armature.bones.iter().find(|bone| bone.id == bone_id) {
            bone_name = bone.name.clone();
        }

        // bind bone label
        ui.label(shared_ui.loc("bone_panel.mesh_deformation.bone_label"));
        let hand = egui::CursorIcon::PointingHand;
        let bone_name = ui.selectable_label(false, bone_name).on_hover_cursor(hand);
        if bone_name.clicked() {
            let bones = &armature.bones;
            let idx = bones.iter().position(|b| b.id == bone_id).unwrap();
            events.select_bone(idx, false);
        };

        // set bind bone toggle
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_set_bone = if edit_mode.setting_bind_bone {
                shared_ui.loc("bone_panel.mesh_deformation.finish")
            } else {
                shared_ui.loc("bone_panel.mesh_deformation.bone_set")
            };
            if ui.skf_button(&str_set_bone).clicked() {
                events.toggle_setting_bind_bone(1);
                // activate bone button flash, to indicate that they must be selected
                if edit_mode.setting_bind_bone {
                    shared_ui.flash_armature_timer = Some(Instant::now());
                }
            }
        });
    });
    let selected = selections.bind as usize;

    let vert_id_len = armature.sel_bone(&sel).unwrap().binds[selected].verts.len();
    if vert_id_len > 0 {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("bone_panel.mesh_deformation.weights_label"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let bind = &armature.sel_bone(&sel).unwrap().binds[selected];
                let mut new_path = bind.is_path;
                ui.checkbox(&mut new_path, "".into_atoms());
                if new_path != bind.is_path {
                    events.toggle_bind_pathing(selected, new_path);
                }

                ui.label(shared_ui.loc("bone_panel.mesh_deformation.pathing_label"))
                    .on_hover_text(shared_ui.loc("bone_panel.mesh_deformation.pathing_desc"));
            });
        });
    }

    let selected = selections.bind;
    let bind = armature.sel_bone(&sel).unwrap().binds[selected as usize].clone();
    let mut is_hovering = false;
    if bind.verts.len() == 0 {
        ui.label(shared_ui.loc("bone_panel.mesh_deformation.no_bound_verts"));
    } else {
        for w in 0..bind.verts.len() {
            ui.horizontal(|ui| {
                let str_label = format!("#{}:", bind.verts[w].id);
                let cursor = egui::CursorIcon::Default;
                if ui.label(str_label).on_hover_cursor(cursor).hovered() {
                    is_hovering = true;
                    events.set_hovering_id(bind.verts[w].id);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut new_weight = bind.verts[w].weight;
                    let slider = ui.add(egui::Slider::new(&mut new_weight, (0.)..=1.));
                    if slider.hovered() {
                        is_hovering = true;
                        events.set_hovering_id(bind.verts[w].id);
                    }
                    if slider.drag_started() {
                        events.save_bone(selections.bone_idx as usize);
                    }
                    if new_weight != bind.verts[w].weight {
                        if !slider.dragged() {
                            events.save_bone(selections.bone_idx as usize);
                        }
                        events.set_bind_weight(selections.bind as u32, w, new_weight);
                    }
                });
            });
        }
    }
    is_hovering
}

pub fn selected_verts_inputs(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    selections: &SelectionState,
    bone: &Bone,
    events: &mut EventState,
) -> bool {
    let mut hovering_id = -1;

    // vertex position inputs
    macro_rules! input {
        ($field:expr, $id:expr, $label:expr, $vert_id:expr, $event:ident, $is_x:expr, $ui:expr) => {
            let init_value = if $is_x { $field.x } else { $field.y };
            let options = Some(crate::ui::TextInputOptions {
                size: Vec2::new(40., 20.),
                drag_modifier: 1.,
                ..Default::default()
            });
            let (edited, value, input) =
                $ui.float_input($id.to_string(), shared_ui, init_value, 1., options);
            if edited {
                events.save_bone(selections.bone_idx);
                let mut new = $field;
                if $is_x {
                    new.x = value;
                } else {
                    new.y = value;
                }
                events.$event($vert_id as u32, new.x, new.y);
            }
            if input.hovered() {
                hovering_id = $vert_id as i32;
            }
            if $ui.label($label.to_string()).hovered() {
                hovering_id = $vert_id as i32;
            }
        };
    }

    for id in &selections.vert_ids {
        macro_rules! with_hover {
            ($widget:expr) => {
                if $widget.hovered() {
                    hovering_id = *id as i32;
                }
            };
        }
        let vert = bone.vertices.iter().find(|v| v.id == *id as u32).unwrap();
        let header_str = shared_ui.loc("bone_panel.mesh_deformation.vertex_header");
        let label_str = format!("{} #{}", header_str, id.to_string());
        let cursor_icon = egui::CursorIcon::Default;
        with_hover!(ui.label(label_str).on_hover_cursor(cursor_icon));
        let pos_inputs = ui.horizontal(|ui| {
            with_hover!(ui.label(shared_ui.loc("bone_panel.mesh_deformation.vert_pos")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let id_x = format!("vert_pos_x{}", id);
                let id_y = format!("vert_pos_y{}", id);
                input!(vert.pos, id_y, "Y:", *id, edit_vertex_pos, false, ui);
                input!(vert.pos, id_x, "X:", *id, edit_vertex_pos, true, ui);
            });
        });
        with_hover!(pos_inputs.response);

        // vertex UV sliders
        let mut new_uv = vert.uv;
        let mut slider1dragged = false;
        let mut slider2dragged = false;
        let u_input = ui.horizontal(|ui| {
            with_hover!(ui.label(shared_ui.loc("bone_panel.mesh_deformation.vert_u")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let slider =
                    ui.add(egui::Slider::new(&mut new_uv.x, (0.)..=1.).update_while_editing(false));
                slider1dragged = slider.dragged();
                if slider.drag_started() {
                    events.save_bone(selections.bone_idx);
                }
                with_hover!(slider);
            });
        });
        with_hover!(u_input.response);
        let v_input = ui.horizontal(|ui| {
            with_hover!(ui.label(shared_ui.loc("bone_panel.mesh_deformation.vert_v")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let slider =
                    ui.add(egui::Slider::new(&mut new_uv.y, (0.)..=1.).update_while_editing(false));
                slider2dragged = slider.dragged();
                if slider.drag_started() {
                    events.save_bone(selections.bone_idx);
                }
                with_hover!(slider);
            });
        });
        with_hover!(v_input.response);

        // update UV values if the sliders have been edited
        if new_uv != vert.uv {
            if !slider1dragged && !slider2dragged {
                events.save_bone(selections.bone_idx);
            }
            events.edit_vertex_uv(*id as u32, new_uv.x, new_uv.y);
        }

        // weight binds for this vertex, if appropriate
        ui.add_space(10.);
        let mut has_binds = false;
        for bi in 0..bone.binds.len() {
            let vert_ids: Vec<usize> = bone.binds[bi].verts.iter().map(|v| v.id as usize).collect();
            has_binds |= vert_ids.contains(id);
        }
        if !has_binds {
            continue;
        }

        ui.label(shared_ui.loc("bone_panel.mesh_deformation.bind_weights_header"));
        for bi in 0..bone.binds.len() {
            // is this vertex in this bind?
            let bind_vert_idx = bone.binds[bi].verts.iter().position(|v| v.id == *id as i32);
            if bind_vert_idx == None {
                continue;
            }

            // bind weight slider
            ui.horizontal(|ui| {
                ui.label(format!("#{}:", bi));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let bind_vert = &bone.binds[bi].verts[bind_vert_idx.unwrap()];
                    let mut new_weight = bind_vert.weight;
                    ui.add(egui::Slider::new(&mut new_weight, (0.)..=1.));
                    if new_weight != bind_vert.weight {
                        events.set_bind_weight(bi as u32, bind_vert_idx.unwrap(), new_weight);
                    }
                });
            });
        }
    }

    // set the vertex being hovered, so it enlarges
    if hovering_id != -1 {
        events.set_hovering_id(hovering_id);
    }

    hovering_id != -1
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_file_dialog(file_path: &Arc<Mutex<Vec<PathBuf>>>, file_type: &Arc<Mutex<i32>>) {
    let filepath = Arc::clone(file_path);
    let filetype = Arc::clone(file_type);
    thread::spawn(move || {
        let task = rfd::FileDialog::new()
            .add_filter("image", &["png", "jpg", "webp"])
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
            ui.label(str_heading);
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

    // texture dropdown
    let mut selected_tex = bone.tex.to_string();
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
                .height(1000.)
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
        let bone_ids = selections.only_root_bones(&armature.bones);
        events.save_edited_bone(selections.bone_idx);
        for id in &bone_ids {
            events.set_bone_texture(*id as usize, selected_tex.clone());
        }
    }

    // don't show other options if not all selected bones have textures
    let mut all_have_tex = true;
    for id in &selections.bone_ids {
        if armature.bones.iter().find(|b| b.id == *id).unwrap().tex == "" {
            all_have_tex = false;
            break;
        }
    }
    if !all_have_tex {
        return;
    }

    ui.horizontal(|ui| {
        ui.label(&shared_ui.loc("bone_panel.zindex"));
        let widgets_width = 70.;
        ui.add_space(ui.available_width() - widgets_width);

        // zindex input
        let zindex = bone.zindex as f32;
        let (edited, value, _) = ui.float_input("zindex".to_string(), shared_ui, zindex, 1., None);
        if edited {
            let el = &AnimElement::Zindex;
            events.save_edited_bone(selections.bone_idx);
            #[rustfmt::skip]
                events.edit_bone(bone.id, el, value, "", selections.anim, selections.anim_frame);
        }

        // raise global zindex button
        let global_str = shared_ui.loc("bone_panel.global_zindex_inc");
        let global_desc_str = shared_ui.loc("bone_panel.global_zindex_inc_desc");
        let button = ui.skf_button(global_str).on_hover_text(global_desc_str);
        if button.clicked() {
            events.raise_global_zindex(bone.id);
        }
    });

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
            let bone_ids = &selections.only_root_bones(&armature.bones);
            edit_bones(bone_ids, E::TintR, col[0], "", anim_id, frame, events);
            edit_bones(bone_ids, E::TintG, col[1], "", anim_id, frame, events);
            edit_bones(bone_ids, E::TintB, col[2], "", anim_id, frame, events);
            edit_bones(bone_ids, E::TintA, col[3], "", anim_id, frame, events);
        });
    });
}

pub fn edit_bones(
    bone_ids: &Vec<i32>,
    element: AnimElement,
    value: f32,
    value_str: &str,
    anim_sel: usize,
    anim_frame: i32,
    events: &mut EventState,
) {
    for id in bone_ids {
        events.edit_bone(*id, &element, value, value_str, anim_sel, anim_frame);
    }
}

pub fn physics(
    ui: &mut egui::Ui,
    bone: &Bone,
    selections: &mut SelectionState,
    shared_ui: &mut crate::Ui,
    config: &Config,
    armature: &Armature,
    events: &mut EventState,
) {
    let sel = selections.clone();
    let str_heading = &shared_ui.loc("bone_panel.physics.heading").clone();
    let str_desc = &shared_ui.loc("bone_panel.physics.desc").clone();
    let frame = egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5));

    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(str_heading).on_hover_text(str_desc);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // todo: add phys_folded
                // don't add until v0.5 nightly is underway, though
                let fold_icon = if bone.ik_folded { "⏴" } else { "⏷" };
                let pointing_hand = egui::CursorIcon::PointingHand;
                if ui.label(fold_icon).on_hover_cursor(pointing_hand).clicked() {
                    let ik_folded = armature.sel_bone(&sel).unwrap().ik_folded;
                    events.toggle_ik_folded(if ik_folded { 0 } else { 1 });
                }
            })
        });
    });
    ui.add_space(2.5);

    if bone.ik_folded {
        return;
    }

    macro_rules! edited {
        ($value:expr, $field:expr, $event:ident) => {
            if $value != $field {
                events.$event($value);
            }
        };
    }

    // pos damping
    #[rustfmt::skip] let pos_damping = phys_slider(bone.phys_pos_damping, "bone_panel.physics.pos_damping", 0., 200., 1., shared_ui, ui);
    edited!(pos_damping, bone.phys_pos_damping, set_pos_damping);
    if bone.phys_pos_damping > 0. {
        // pos ratio
        #[rustfmt::skip] let pos_ratio = phys_sub_slider(bone.phys_pos_ratio, "bone_panel.physics.pos_ratio", -1., 1., true, shared_ui, ui);
        edited!(pos_ratio, bone.phys_pos_ratio, set_pos_ratio);
    }

    // scale damping
    #[rustfmt::skip] let scale_damping = phys_slider(bone.phys_scale_damping, "bone_panel.physics.scale_damping", 0., 200., 1., shared_ui, ui);
    edited!(scale_damping, bone.phys_scale_damping, set_scale_damping);
    if bone.phys_scale_damping > 0. {
        // scale ratio
        #[rustfmt::skip] let scale_ratio = phys_sub_slider(bone.phys_scale_ratio, "bone_panel.physics.scale_ratio", -1., 1., true, shared_ui, ui);
        edited!(scale_ratio, bone.phys_scale_ratio, set_scale_ratio);
    }

    // rot damping
    #[rustfmt::skip] let rot_damping = phys_slider(bone.phys_rot_damping, "bone_panel.physics.rot_damping", 0., 200., 1., shared_ui, ui);
    edited!(rot_damping, bone.phys_rot_damping, set_rot_damping);

    if bone.parent_id != -1 {
        // sway
        #[rustfmt::skip] let sway = phys_slider(bone.phys_sway, "bone_panel.physics.sway", 0., 10., 0.1, shared_ui, ui);
        edited!(sway, bone.phys_sway, set_rot_resistance);

        if bone.phys_sway > 0. {
            // bounce
            #[rustfmt::skip] let bounce = phys_slider(bone.phys_rot_bounce, "bone_panel.physics.rot_bounce", 0., 1., 0.01, shared_ui, ui);
            edited!(bounce, bone.phys_rot_bounce, set_rot_bounce);
        }
    }
}

pub fn phys_slider(
    field: f32,
    label_code: &str,
    min: f32,
    max: f32,
    drag_modifier: f32,
    shared_ui: &mut crate::Ui,
    ui: &mut egui::Ui,
) -> f32 {
    let mut result = field;

    ui.horizontal(|ui| {
        let loc = shared_ui.loc(&label_code).to_string();
        let label = ui.label(loc);

        // show tooltip, if it exists
        let desc_code = &format!("{}{}", label_code, "_desc");
        let desc_loc = shared_ui.loc(desc_code).to_string();
        if desc_loc != *desc_code {
            label.on_hover_text(desc_loc);
        }

        // text input
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let options = ui::TextInputOptions {
                size: Vec2::new(0., 0.),
                drag_modifier,
                ..Default::default()
            };
            let (edited, value, _) =
                ui.float_input(label_code.to_string(), shared_ui, field, 1., Some(options));
            if edited {
                result = value;
            }
        });
    });

    // slider
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.style_mut().spacing.slider_width = ui.available_width();
            let mut new_field = field;
            let slider = ui.add(egui::Slider::new(&mut new_field, min..=max).show_value(false));

            // result must be updated only if slider is dragged
            if slider.dragged() {
                if field > max {
                    new_field = max;
                }
                result = new_field;
            }
        });
    });

    result
}

/// Physics fields that relate to another, eg; ratio.
pub fn phys_sub_slider(
    field: f32,
    label_code: &str,
    min: f32,
    max: f32,
    is_ratio: bool,
    shared_ui: &mut crate::Ui,
    ui: &mut egui::Ui,
) -> f32 {
    let mut result = field;

    ui.horizontal(|ui| {
        ui.add_space(30.);
        let loc = shared_ui.loc(&label_code).to_string();
        let label = ui.label(loc);

        // show tooltip, if it exists
        let desc_code = &format!("{}{}", label_code, "_desc");
        let desc_loc = shared_ui.loc(desc_code).to_string();
        if desc_loc != *desc_code {
            label.on_hover_text(desc_loc);
        }

        // text input
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (edited, value, _) =
                ui.float_input(label_code.to_string(), shared_ui, field, 1., None);
            if edited {
                result = value;
            }

            if is_ratio {
                // show X:Y ratio
                let mut ratio = Vec2::new(1., 1.);
                if field < 0. {
                    ratio.y = 1. - field.abs();
                } else if field > 0. {
                    ratio.x = 1. - field;
                }
                ui.label(format!("{:.2} : {:.2}", ratio.x, ratio.y));
            }
        });
    });

    // slider
    ui.horizontal(|ui| {
        ui.add_space(30.);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.style_mut().spacing.slider_width = ui.available_width();
            let mut new_field = field;
            let slider = ui.add(egui::Slider::new(&mut new_field, min..=max).show_value(false));

            // result must be updated only if slider is dragged
            if slider.dragged() {
                if field > max {
                    new_field = max;
                }
                result = new_field;
            }
        });
    });

    result
}
