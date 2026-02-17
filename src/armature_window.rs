//! UI Armature window.

use egui::*;

use crate::{
    shared::Vec2,
    ui::{self, EguiUi, TextInputOptions},
    utils,
};

use crate::shared::*;

pub fn draw(
    egui_ctx: &Context,
    events: &mut EventState,
    config: &Config,
    armature: &Armature,
    selections: &SelectionState,
    edit_mode: &EditMode,
    shared_ui: &mut crate::Ui,
) {
    let min_default_size = 175.;
    let panel_id = "Armature";
    let side_panel: egui::SidePanel;
    match config.layout {
        UiLayout::Split | UiLayout::Left => {
            side_panel = egui::SidePanel::left(panel_id)
                .default_width(min_default_size)
                .min_width(min_default_size)
                .max_width(min_default_size + 100.)
                .resizable(true);
        }
        UiLayout::Right => {
            side_panel = egui::SidePanel::right(panel_id)
                .default_width(min_default_size)
                .min_width(min_default_size)
                .max_width(min_default_size + 100.)
                .resizable(true);
        }
    }

    let panel = side_panel.resizable(true).show(egui_ctx, |ui| {
        if shared_ui.startup_window {
            shared_ui.armature_panel_rect = Some(ui.min_rect());
            return;
        }
        let gradient = config.colors.gradient.into();
        ui.gradient(ui.ctx().content_rect(), Color32::TRANSPARENT, gradient);
        ui.horizontal(|ui| {
            ui.heading(&shared_ui.loc("armature_panel.heading"));
        });

        ui.separator();

        ui.horizontal(|ui| {
            let button = ui.skf_button(&&shared_ui.loc("armature_panel.new_bone_button"));
            if button.clicked() {
                events.new_bone();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if armature.bones.len() == 0 {
                    return;
                }
                let dropdown = egui::ComboBox::new("styles", "")
                    .selected_text(&shared_ui.loc("armature_panel.styles"))
                    .width(80.)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show_ui(ui, |ui| {
                        for s in 0..armature.styles.len() {
                            ui.set_width(80.);
                            let active = armature.styles[s].active;
                            let tick = if active { " 👁" } else { "" };
                            let mut name = armature.styles[s].name.to_string();
                            name = utils::trunc_str(ui, &name, ui.min_rect().width() - 20.);
                            let label = ui.selectable_value(&mut -1, s as i32, name);
                            ui.painter().text(
                                label.rect.right_center(),
                                egui::Align2::RIGHT_CENTER,
                                tick,
                                egui::FontId::default(),
                                config.colors.text.into(),
                            );
                            if label.clicked() {
                                events.toggle_style_active(s, !armature.styles[s].active);
                            }
                        }
                        let label = ui.selectable_value(&mut -1, -2, "[Setup]");
                        if label.clicked() {
                            shared_ui.styles_modal = true;
                            ui.close();
                        }
                    })
                    .response
                    .on_hover_text(&shared_ui.loc("armature_panel.styles_desc"));

                if shared_ui.focus_style_dropdown {
                    dropdown.request_focus();
                    shared_ui.focus_style_dropdown = false;
                }
            });
        });
        ui.add_space(3.);
        let scroll_area = egui::ScrollArea::both().max_height(ui.available_height() - 10.);
        scroll_area.show(ui, |ui| {
            // hierarchy
            let frame = Frame::default().inner_margin(5.);
            ui.dnd_drop_zone::<i32, _>(frame, |ui| {
                ui.set_min_height(ui.available_height());
                ui.set_width(ui.available_width());

                // The empty armature text should have blue hyperlinks to attract the user's
                // attention. The blue makes it clear of being a hyperlink, while also sticking
                // out (without being too jarring).
                ui.style_mut().visuals.hyperlink_color = egui::Color32::from_rgb(94, 156, 255);

                if armature.bones.len() != 0 {
                    #[rustfmt::skip]
                    draw_hierarchy(ui, shared_ui, &selections, &armature, &config, &edit_mode, events);
                } else {
                    let mut cache = egui_commonmark::CommonMarkCache::default();
                    let armature_str = shared_ui.loc("armature_panel.empty_armature");
                    let str = utils::markdown(armature_str, shared_ui.local_doc_url.clone());
                    egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
                }
                ui.add_space(4.);
            });
        });
        shared_ui.armature_panel_rect = Some(ui.min_rect());
    });

    ui::draw_resizable_panel(panel_id, panel, events, &egui_ctx);
}

pub fn draw_hierarchy(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    selections: &SelectionState,
    armature: &Armature,
    config: &Config,
    edit_mode: &EditMode,
    events: &mut EventState,
) {
    ui.set_min_width(ui.available_width());
    let mut idx: i32 = -1;
    let mut is_hovering = false;
    let sel = selections.clone();

    for b in 0..armature.bones.len() {
        idx += 1;
        if armature.is_bone_folded(armature.bones[b].id) {
            continue;
        }
        let bone_id = armature.bones[b].id;

        let mut dragged = false;

        let parents = armature.get_all_parents(false, armature.bones[b].id);
        let bone = armature.sel_bone(&sel);
        let selected_bone_id = if bone != None { bone.unwrap().id } else { -1 };

        // disable selected bone and it's children from armature if setting IK target,
        // since IK target cannot be itself
        let setting_ik_target = edit_mode.setting_ik_target
            && (bone_id == selected_bone_id
                || parents.iter().find(|bone| bone.id == selected_bone_id) != None);

        ui.add_enabled_ui(!setting_ik_target, |ui| {
            ui.horizontal(|ui| {
                let id = "bone_hidden".to_owned() + &b.to_string();
                let hidden = armature.is_bone_hidden(true, config.propagate_visibility, bone_id);
                let mut col = config.colors.text;
                if hidden {
                    col -= Color::new(80, 80, 80, 0);
                }
                if bone_label("👁", ui, id, Vec2::new(-2., 18.), col).clicked() {
                    let hidden_f32 = if !hidden { 1. } else { 0. };
                    let sel = selections.anim;
                    let frame = selections.anim_frame;
                    events.save_edited_bone(b);
                    events.edit_bone(bone_id, &AnimElement::Hidden, hidden_f32, sel, frame);
                }
                let locked = armature.bones[b].locked;
                let mut col = config.colors.text;
                if !locked {
                    col -= Color::new(80, 80, 80, 0);
                }
                let id = "bone_locked".to_owned() + &b.to_string();
                if bone_label("🔒", ui, id, Vec2::new(15., 18.), col).clicked() {
                    let locked_f32 = if !locked { 1. } else { 0. };
                    events.save_edited_bone(b);
                    events.edit_bone(bone_id, &AnimElement::Locked, locked_f32, usize::MAX, -1);
                }
                ui.add_space(34.);

                // add space to the left if this is a child
                for _ in 0..parents.len() {
                    vert_line(0., ui, &config);
                    ui.add_space(15.);
                }

                // show folding button if this bone has children
                let mut children = vec![];
                let bone = &armature.bones[b];
                get_all_children(&armature.bones, &mut children, bone);
                if children.len() == 0 {
                    hor_line(11., ui, &config);
                } else {
                    let folded = armature.bones[b].folded;
                    let fold_icon = if folded { "⏵" } else { "⏷" };
                    let id = "bone_fold".to_owned() + &b.to_string();
                    if bone_label(fold_icon, ui, id, Vec2::new(-2., 18.), config.colors.text)
                        .clicked()
                    {
                        events.toggle_bone_folded(idx as usize, !armature.bones[b].folded);
                    }
                }
                ui.add_space(13.);

                let mut selected_col = config.colors.dark_accent;
                let mut cursor = egui::CursorIcon::PointingHand;

                if hidden {
                    selected_col = config.colors.dark_accent;
                }

                if shared_ui.hovering_bone == idx {
                    selected_col += Color::new(20, 20, 20, 0);
                }

                let id = &armature.bones[idx as usize].id;
                let is_multi_selected = selections.bone_ids.contains(id);
                if selections.bone_idx == idx as usize || is_multi_selected {
                    selected_col += Color::new(20, 20, 20, 0);
                    cursor = egui::CursorIcon::Default;
                }

                let width = ui.available_width();
                let context_id = "bone_".to_string() + &idx.to_string();
                if shared_ui.rename_id == context_id {
                    let bone_name = shared_ui.loc("armature_panel.new_bone_name").to_string();
                    let bone = armature.bones[b].name.clone();
                    let options = Some(TextInputOptions {
                        size: Vec2::new(ui.available_width(), 21.),
                        focus: true,
                        placeholder: bone_name.clone(),
                        default: bone_name,
                        ..Default::default()
                    });
                    let (edited, value, _) = ui.text_input(context_id, shared_ui, bone, options);
                    if edited {
                        events.save_bone(idx as usize);
                        events.rename_bone(idx as usize, value);
                    }
                    return;
                }

                let id = Id::new(("bone", idx, 0));
                let button = ui
                    .dnd_drag_source(id, idx, |ui| {
                        ui.set_width(width);
                        let name = armature.bones[b].name.to_string();
                        let mut text_col = config.colors.text;
                        if hidden {
                            text_col = config.colors.dark_accent;
                            text_col += Color::new(40, 40, 40, 0)
                        }
                        egui::Frame::new().fill(selected_col.into()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_width(width);
                                ui.set_height(21.);
                                ui.add_space(5.);
                                ui.label(egui::RichText::new(name).color(text_col));

                                let mut icon_col = config.colors.dark_accent;
                                icon_col += Color::new(40, 40, 40, 0);

                                let is_target = armature
                                    .bones
                                    .iter()
                                    .find(|b| b.ik_family_id != -1 && b.ik_target_id == bone.id);

                                ui.spacing_mut().item_spacing.x = 5.;
                                ui.style_mut().visuals.override_text_color = Some(icon_col.into());
                                ui.style_mut()
                                    .text_styles
                                    .insert(egui::TextStyle::Body, egui::FontId::monospace(14.0));

                                if armature.tex_of(bone.id) != None {
                                    let str = shared_ui
                                        .loc("armature_panel.icons.tex")
                                        .replace("$tex", &bone.tex);
                                    icon_label(ui, "🖻", str, config.colors.texture);
                                } else if bone.tex != "" {
                                    let str = shared_ui
                                        .loc("armature_panel.icons.tex_inactive")
                                        .replace("$tex", &bone.tex);
                                    icon_label(ui, "🗋", str, config.colors.texture);
                                };
                                if bone.verts_edited {
                                    let mesh_str = shared_ui.loc("armature_panel.icons.mesh");
                                    icon_label(ui, "⬟", mesh_str, config.colors.meshdef);
                                }
                                if bone.ik_family_id != -1 {
                                    let icon = "🔧".to_owned() + &bone.ik_family_id.to_string();
                                    let desc = shared_ui
                                        .loc("armature_panel.icons.ik_family")
                                        .replace("$family_id", &bone.ik_family_id.to_string());
                                    icon_label(ui, &icon, desc, config.colors.inverse_kinematics);
                                }
                                if is_target != None {
                                    let family_id = is_target.unwrap().ik_family_id.to_string();
                                    let icon = "⌖".to_owned() + &family_id;
                                    let desc = shared_ui
                                        .loc("armature_panel.icons.ik_target")
                                        .replace("$family_id", &family_id);
                                    let inc = 20 * is_target.unwrap().ik_family_id as u8;
                                    let mut color = config.colors.ik_target;
                                    color += Color::new(0, inc, inc, 0);
                                    icon_label(ui, &icon, desc, color);
                                }
                            });
                        });
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_cursor(cursor);

                if button.contains_pointer() || button.has_focus() {
                    is_hovering = true;
                    shared_ui.hovering_bone = idx;
                }

                if button.clicked() {
                    events.select_bone(idx as usize, false);
                }

                crate::context_menu!(button, shared_ui, context_id, |ui: &mut egui::Ui| {
                    ui.context_rename(shared_ui, &config, context_id.clone());
                    let delete_bone = PolarId::DeleteBone;
                    ui.context_delete(shared_ui, &config, events, "delete_bone", delete_bone);

                    if ui.context_button("Copy", &config).clicked() {
                        events.copy_bone(b);
                        shared_ui.context_menu.close();
                    }

                    if ui.context_button("Paste", &config).clicked() {
                        events.paste_bone(b);
                        shared_ui.context_menu.close();
                    }
                });

                if check_bone_dragging(events, &armature, ui, button, idx as usize) {
                    dragged = true;
                }
            });
        });

        if dragged {
            break;
        }
    }

    if !is_hovering {
        shared_ui.hovering_bone = -1;
    }
}

pub fn bone_label(
    icon: &str,
    ui: &mut egui::Ui,
    id: String,
    offset: Vec2,
    color: Color,
) -> egui::Response {
    let rect = ui.painter().text(
        ui.cursor().min + Vec2::new(offset.x, offset.y).into(),
        egui::Align2::LEFT_BOTTOM,
        icon,
        egui::FontId::default(),
        color.into(),
    );
    ui.interact(rect, id.into(), egui::Sense::CLICK)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn check_bone_dragging(
    events: &mut EventState,
    armature: &Armature,
    ui: &mut egui::Ui,
    drag: Response,
    idx: usize,
) -> bool {
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let hovered_payload = drag.dnd_hover_payload::<i32>();
    let rect = drag.rect;
    let stroke = egui::Stroke::new(1.0, Color32::WHITE);

    if pointer == None || hovered_payload == None {
        return false;
    }

    // prevent dragging bone onto itself
    if *hovered_payload.unwrap() == idx as i32 {
        return false;
    }

    let mut is_above = false;

    if pointer.unwrap().y < rect.center().y {
        // above bone (move dragged bone above it)
        ui.painter().hline(rect.x_range(), rect.top(), stroke);
        is_above = true;
    } else {
        // in bone (turn dragged bone to child)
        ui.painter().hline(rect.x_range(), rect.top(), stroke);
        ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
        ui.painter().vline(rect.right(), rect.y_range(), stroke);
        ui.painter().vline(rect.left(), rect.y_range(), stroke);
    };

    let drag_payload = drag.dnd_release_payload::<i32>();
    if drag_payload == None {
        return false;
    };

    events.drag_bone(
        is_above,
        armature.bones[idx].id as usize,
        armature.bones[*drag_payload.unwrap() as usize].id as usize,
    );
    return true;
}

pub fn vert_line(offset: f32, ui: &mut egui::Ui, config: &Config) {
    let rect = egui::Rect::from_min_size(
        ui.cursor().left_top() + [3., -1.5 + offset].into(),
        [2., 24.].into(),
    );
    let mut line_col = config.colors.dark_accent;
    line_col += Color::new(20, 20, 20, 0);
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, line_col);
}

pub fn hor_line(offset: f32, ui: &mut egui::Ui, config: &Config) {
    let rect = egui::Rect::from_min_size(
        ui.cursor().left_top() + [-2., -1.5 + offset].into(),
        [12., 2.].into(),
    );
    let mut line_col = config.colors.dark_accent;
    line_col += Color::new(20, 20, 20, 0);
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, line_col);
}

/// Retrieve all children of this bone (recursive)
pub fn get_all_children(bones: &Vec<Bone>, children_vec: &mut Vec<Bone>, parent: &Bone) {
    let idx = bones.iter().position(|b| b.id == parent.id).unwrap();

    for j in 1..(bones.len() - idx) {
        if bones[idx + j].parent_id != parent.id {
            continue;
        }
        children_vec.push(bones[idx + j].clone());
        get_all_children(bones, children_vec, &bones[idx + j]);
    }
}

fn icon_label(ui: &mut egui::Ui, icon: &str, desc: String, color: Color) {
    let label = ui.label(egui::RichText::new(icon).color(color));
    if label.contains_pointer() {
        label.show_tooltip_text(desc);
    }
}
