//! UI Armature window.

use egui::*;

use crate::{
    shared::{Shared, Vec2},
    ui::{self, EguiUi, TextInputOptions},
    utils,
};

use crate::shared::*;

pub fn draw(egui_ctx: &Context, shared: &mut Shared) {
    let min_default_size = 175.;
    let panel_id = "Armature";
    let side_panel = egui::SidePanel::left(panel_id)
        .default_width(min_default_size)
        .min_width(min_default_size)
        .max_width(min_default_size + 100.)
        .resizable(true);
    ui::draw_resizable_panel(
        panel_id,
        side_panel.resizable(true).show(egui_ctx, |ui| {
            ui.gradient(
                ui.ctx().screen_rect(),
                Color32::TRANSPARENT,
                shared.config.colors.gradient.into(),
            );
            ui.horizontal(|ui| {
                ui.heading(&shared.loc("armature_panel.heading"));
            });

            ui.separator();

            ui.horizontal(|ui| {
                let button = ui.skf_button(&&shared.loc("armature_panel.new_bone_button"));
                if button.clicked() {
                    let idx: usize;

                    shared.undo_actions.push(Action {
                        action: ActionType::Bones,
                        bones: shared.armature.bones.clone(),
                        ..Default::default()
                    });

                    if shared.selected_bone() == None {
                        (_, idx) = shared.armature.new_bone(-1);
                    } else {
                        let id = shared.selected_bone().unwrap().id;
                        (_, idx) = shared.armature.new_bone(id);
                    }

                    // immediately select new bone upon creating it
                    shared.ui.select_bone(idx);

                    shared.selected_bone_mut().unwrap().name =
                        shared.loc("armature_panel.new_bone_name");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if shared.armature.bones.len() == 0 {
                        return;
                    }
                    let mut selected_style = -1;
                    let dropdown = egui::ComboBox::new("styles", "")
                        .selected_text(&shared.loc("armature_panel.styles"))
                        .width(80.)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show_ui(ui, |ui| {
                            for s in 0..shared.armature.styles.len() {
                                ui.set_width(80.);
                                let tick = if shared.armature.styles[s].active {
                                    " 👁"
                                } else {
                                    ""
                                };
                                let label = ui.selectable_value(
                                    &mut selected_style,
                                    s as i32,
                                    shared.armature.styles[s].name.to_string(),
                                );
                                ui.painter().text(
                                    label.rect.right_center(),
                                    egui::Align2::RIGHT_CENTER,
                                    tick,
                                    egui::FontId::default(),
                                    shared.config.colors.text.into(),
                                );
                                if label.clicked() {
                                    shared.armature.styles[s].active =
                                        !shared.armature.styles[s].active;
                                }
                            }
                            let label = ui.selectable_value(&mut selected_style, -2, "[Setup]");
                            if label.clicked() {
                                ui.close();
                            }
                        })
                        .response
                        .on_hover_text(&shared.loc("armature_panel.styles_desc"));

                    if shared.ui.has_state(UiState::FocusStyleDropdown) {
                        dropdown.request_focus();
                        shared.ui.set_state(UiState::FocusStyleDropdown, false);
                    }
                    if selected_style == -2 {
                        shared.open_style_modal();
                    } else if selected_style != -1 {
                        shared.ui.selected_style = selected_style;
                        for b in 0..shared.armature.bones.len() {
                            if shared.armature.bones[b].style_ids.contains(&selected_style) {
                                shared.armature.set_bone_tex(
                                    shared.armature.bones[b].id,
                                    shared.armature.bones[b].tex_idx as usize,
                                    shared.ui.anim.selected,
                                    shared.ui.anim.selected_frame,
                                );
                            }
                        }
                    }
                });
            });

            shared.ui.edit_bar_pos.x = ui.min_rect().right();

            ui.add_space(3.);

            egui::ScrollArea::both()
                .max_height(ui.available_height() - 10.)
                .show(ui, |ui| {
                    // hierarchy
                    let frame = Frame::default().inner_margin(5.);
                    ui.dnd_drop_zone::<i32, _>(frame, |ui| {
                        ui.set_height(ui.available_height());
                        ui.set_width(ui.available_width());

                        // The empty armature text should have blue hyperlinks to attract the user's
                        // attention. The blue makes it clear of being a hyperlink, while also sticking
                        // out (without being too jarring).
                        ui.style_mut().visuals.hyperlink_color =
                            egui::Color32::from_rgb(94, 156, 255);

                        if shared.armature.bones.len() != 0 {
                            draw_hierarchy(shared, ui);
                        } else {
                            let mut cache = egui_commonmark::CommonMarkCache::default();
                            let str = utils::markdown(
                                shared.loc("bone_panel.empty_armature"),
                                shared.local_doc_url.clone(),
                            );
                            egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
                        }
                        ui.add_space(4.);
                    });
                });
        }),
        &mut shared.input.on_ui,
        &egui_ctx,
    );
}

pub fn draw_hierarchy(shared: &mut Shared, ui: &mut egui::Ui) {
    ui.set_min_width(ui.available_width());
    let mut idx: i32 = -1;
    let mut is_hovering = false;

    for b in 0..shared.armature.bones.len() {
        idx += 1;
        // if this bone's parent is folded, skip drawing
        let mut visible = true;
        let mut nb = &shared.armature.bones[b];
        while nb.parent_id != -1 {
            nb = shared.armature.find_bone(nb.parent_id).unwrap();
            if nb.folded {
                visible = false;
                break;
            }
        }
        if !visible {
            continue;
        }
        let bone_id = shared.armature.bones[b].id;

        let mut dragged = false;

        let parents = shared.armature.get_all_parents(shared.armature.bones[b].id);
        let selected_bone_id = if let Some(bone) = shared.selected_bone() {
            bone.id
        } else {
            -1
        };

        // disable selected bone and it's children from armature if setting IK target,
        // since IK target cannot be itself
        let setting_ik_target = shared.ui.setting_ik_target
            && (bone_id == selected_bone_id
                || parents.iter().find(|bone| bone.id == selected_bone_id) != None);

        ui.add_enabled_ui(!setting_ik_target, |ui| {
            ui.horizontal(|ui| {
                let hidden_icon = if shared.armature.is_bone_hidden(bone_id) {
                    "---"
                } else {
                    "👁"
                };
                let id = "bone_hidden".to_owned() + &b.to_string();
                if bone_label(hidden_icon, ui, id, shared, Vec2::new(-2., 18.)).clicked() {
                    shared.armature.bones[b].hidden = !shared.armature.bones[b].hidden;
                }
                ui.add_space(17.);

                // add space to the left if this is a child
                for _ in 0..parents.len() {
                    vert_line(0., ui, shared);
                    ui.add_space(15.);
                }

                // show folding button if this bone has children
                let mut children = vec![];
                get_all_children(
                    &shared.armature.bones,
                    &mut children,
                    &shared.armature.bones[b],
                );
                if children.len() == 0 {
                    hor_line(11., ui, shared);
                } else {
                    let fold_icon = if shared.armature.bones[b].folded {
                        "⏵"
                    } else {
                        "⏷"
                    };
                    let id = "bone_fold".to_owned() + &b.to_string();
                    if bone_label(fold_icon, ui, id, shared, Vec2::new(-2., 18.)).clicked() {
                        shared.armature.bones[b].folded = !shared.armature.bones[b].folded;
                    }
                }
                ui.add_space(13.);

                let mut selected_col = shared.config.colors.dark_accent;
                let mut cursor = egui::CursorIcon::PointingHand;

                if shared.armature.is_bone_hidden(bone_id) {
                    selected_col = shared.config.colors.dark_accent;
                }

                if shared.ui.hovering_bone == idx {
                    selected_col += Color::new(20, 20, 20, 0);
                }

                let is_multi_selected = shared
                    .ui
                    .selected_bone_ids
                    .contains(&(shared.armature.bones[idx as usize].id));

                if shared.ui.selected_bone_idx == idx as usize || is_multi_selected {
                    selected_col += Color::new(20, 20, 20, 0);
                    cursor = egui::CursorIcon::Default;
                }

                let width = ui.available_width();
                let rename_str = "bone_".to_string() + &idx.to_string();

                if shared.ui.rename_id == rename_str {
                    let (edited, value, _) = ui.text_input(
                        shared.ui.rename_id.clone(),
                        shared,
                        shared.ui.edit_value.clone().unwrap(),
                        Some(TextInputOptions {
                            size: Vec2::new(ui.available_width(), 21.),
                            focus: true,
                            ..Default::default()
                        }),
                    );
                    if edited {
                        shared.selected_bone_mut().unwrap().name = value;
                    }
                    return;
                }

                let id = Id::new(("bone", idx, 0));
                let button = ui
                    .dnd_drag_source(id, idx, |ui| {
                        ui.set_width(width);

                        let name = shared.armature.bones[b].name.to_string();
                        let mut text_col = shared.config.colors.text;
                        if shared.armature.is_bone_hidden(shared.armature.bones[b].id) {
                            text_col = shared.config.colors.dark_accent;
                            text_col += Color::new(40, 40, 40, 0)
                        }
                        egui::Frame::new().fill(selected_col.into()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_width(width);
                                ui.set_height(21.);
                                ui.add_space(5.);
                                ui.label(egui::RichText::new(name).color(text_col));

                                let has_tex = shared.armature.get_current_tex(bone_id) != None;

                                let pic = if has_tex { "🖻  " } else { "" };
                                let mut pic_col = shared.config.colors.dark_accent;
                                pic_col += Color::new(40, 40, 40, 0);
                                ui.label(egui::RichText::new(pic).color(pic_col))
                            });
                        });
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_cursor(cursor);

                if button.contains_pointer() {
                    is_hovering = true;
                    shared.ui.hovering_bone = idx;
                }

                if button.clicked() {
                    if shared.ui.selected_bone_idx == idx as usize {
                        shared.ui.rename_id = rename_str;
                        shared.ui.edit_value = Some(shared.armature.bones[b].name.clone());
                    } else {
                        if shared.ui.setting_ik_target {
                            shared.selected_bone_mut().unwrap().ik_target_id = bone_id;
                            shared.ui.setting_ik_target = false;
                        } else {
                            if !shared.input.holding_mod && !shared.input.holding_shift {
                                shared.ui.selected_bone_ids = vec![];
                                let anim_frame = shared.ui.anim.selected_frame;
                                shared.ui.select_bone(idx as usize);
                                shared.ui.anim.selected_frame = anim_frame;
                            }

                            let id = shared.armature.bones[idx as usize].id;
                            shared.ui.selected_bone_ids.push(id);

                            if shared.input.holding_shift {
                                let mut first = shared.ui.selected_bone_idx;
                                let mut second = idx as usize;
                                if first > second {
                                    first = idx as usize;
                                    second = shared.ui.selected_bone_idx;
                                }
                                for i in first..second as usize {
                                    let bone = &shared.armature.bones[i];
                                    if !shared.ui.selected_bone_ids.contains(&bone.id)
                                        && bone.parent_id
                                            == shared.selected_bone().unwrap().parent_id
                                    {
                                        shared.ui.selected_bone_ids.push(bone.id);
                                    }
                                }
                            }
                        }
                    }
                }

                let id = shared.armature.bones[b].id;

                if button.secondary_clicked() {
                    shared.ui.context_menu.show(ContextType::Bone, id);
                }

                if shared.ui.context_menu.is(ContextType::Bone, id) {
                    button.show_tooltip_ui(|ui| {
                        if ui.clickable_label("Delete").clicked() {
                            let str_del = &shared.loc("polar.delete_bone").clone();
                            shared.ui.open_polar_modal(PolarId::DeleteBone, &str_del);
                            shared.ui.context_menu.hide = true;
                        };

                        if ui.clickable_label("Copy").clicked() {
                            shared.copy_buffer = CopyBuffer::default();
                            let mut bones = vec![];
                            get_all_children(
                                &shared.armature.bones,
                                &mut bones,
                                &shared.armature.bones[b],
                            );
                            bones.insert(0, shared.armature.bones[b].clone());
                            shared.copy_buffer.bones = bones;
                            shared.ui.context_menu.close();
                        }

                        if ui.ui_contains_pointer() {
                            shared.ui.context_menu.keep = true;
                        }
                    });
                }

                if check_bone_dragging(shared, ui, button, idx as usize) {
                    dragged = true;
                }
            });
        });

        if dragged {
            break;
        }
    }

    if !is_hovering {
        shared.ui.hovering_bone = -1;
    }
}

pub fn bone_label(
    icon: &str,
    ui: &mut egui::Ui,
    id: String,
    shared: &Shared,
    offset: Vec2,
) -> egui::Response {
    let rect = ui.painter().text(
        ui.cursor().min + Vec2::new(offset.x, offset.y).into(),
        egui::Align2::LEFT_BOTTOM,
        icon,
        egui::FontId::default(),
        shared.config.colors.text.into(),
    );
    ui.interact(rect, id.into(), egui::Sense::CLICK)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn check_bone_dragging(shared: &mut Shared, ui: &mut egui::Ui, drag: Response, idx: usize) -> bool {
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
    let drag_idx = drag_payload.unwrap();

    // if this bone wasn't selected before being dragged,
    // set only this one to be dragged.
    // prevents edge case of dragging bones while another is selected.
    let drag_id = shared.armature.bones[*drag_idx as usize].id;
    if !shared.ui.selected_bone_ids.contains(&drag_id) {
        shared.ui.selected_bone_ids = vec![drag_id];
    }

    let pointing_id = shared.armature.bones[idx].id;

    // ignore if pointing bone is also selected
    if shared.ui.selected_bone_ids.contains(&pointing_id) {
        return false;
    }

    // ignore if pointing bone is a child of this
    let mut children: Vec<Bone> = vec![];
    let first_drag_bone = shared
        .armature
        .find_bone(shared.ui.selected_bone_ids[0])
        .unwrap();
    get_all_children(&shared.armature.bones, &mut children, &first_drag_bone);
    for c in children {
        if shared.armature.bones[idx as usize].id == c.id {
            return false;
        }
    }

    shared.ui.selected_bone_idx = usize::MAX;

    shared.undo_actions.push(Action {
        action: ActionType::Bones,
        bones: shared.armature.bones.clone(),
        ..Default::default()
    });

    // sort dragged bones so they'll appear in the same order when dropped
    let mut sorted_ids = shared.ui.selected_bone_ids.clone();
    sorted_ids.sort_by(|a, b| {
        let mut first = *b;
        let mut second = *a;
        if is_above {
            first = *a;
            second = *b;
        }
        let first_idx = shared.armature.find_bone_idx(first).unwrap();
        let second_idx = shared.armature.find_bone_idx(second).unwrap();
        first_idx.cmp(&second_idx)
    });

    for id in sorted_ids {
        let old_parents = shared.armature.get_all_parents(id as i32);
        drag_bone(shared, is_above, id, pointing_id);
        shared.armature.offset_pos_by_parent(old_parents, id as i32);
    }

    return true;
}

pub fn drag_bone(shared: &mut Shared, is_above: bool, dragged_id: i32, pointing_id: i32) {
    macro_rules! dragged {
        () => {
            shared.armature.find_bone_mut(dragged_id).unwrap()
        };
    }
    macro_rules! pointing {
        () => {
            shared.armature.find_bone_mut(pointing_id).unwrap()
        };
    }

    let dragged_idx = shared.armature.find_bone_idx(dragged_id).unwrap();
    let pointing_idx = shared.armature.find_bone_idx(pointing_id).unwrap();

    if is_above {
        // set pointed bone's parent as dragged bone's parent
        dragged!().parent_id = pointing!().parent_id;
        move_bone(
            &mut shared.armature.bones,
            dragged_idx as i32,
            pointing_idx as i32,
            false,
        );
    } else {
        // set pointed bone as dragged bone's parent
        dragged!().parent_id = pointing!().id;
        move_bone(
            &mut shared.armature.bones,
            dragged_idx as i32,
            pointing_idx as i32,
            true,
        );

        pointing!().folded = false;
    }
}

pub fn move_bone(bones: &mut Vec<Bone>, old_idx: i32, new_idx: i32, is_setting_parent: bool) {
    let main = &bones[old_idx as usize];
    let anchor = bones[new_idx as usize].clone();

    // gather all bones to be moved (this and its children)
    let mut to_move: Vec<Bone> = vec![main.clone()];
    get_all_children(bones, &mut to_move, main);

    // remove them
    for _ in &to_move {
        bones.remove(old_idx as usize);
    }

    // re-add them in the new positions
    if is_setting_parent {
        to_move.reverse();
    }
    for bone in to_move {
        bones.insert(
            find_bone_idx(bones, anchor.id) as usize + is_setting_parent as usize,
            bone.clone(),
        );
    }
}

pub fn vert_line(offset: f32, ui: &mut egui::Ui, shared: &mut Shared) {
    let rect = egui::Rect::from_min_size(
        ui.cursor().left_top() + [3., -1.5 + offset].into(),
        [2., 24.].into(),
    );
    let mut line_col = shared.config.colors.dark_accent;
    line_col += Color::new(20, 20, 20, 0);
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, line_col);
}

pub fn hor_line(offset: f32, ui: &mut egui::Ui, shared: &mut Shared) {
    let rect = egui::Rect::from_min_size(
        ui.cursor().left_top() + [-2., -1.5 + offset].into(),
        [12., 2.].into(),
    );
    let mut line_col = shared.config.colors.dark_accent;
    line_col += Color::new(20, 20, 20, 0);
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, line_col);
}

/// Retrieve all children of this bone (recursive)
pub fn get_all_children(bones: &Vec<Bone>, children_vec: &mut Vec<Bone>, parent: &Bone) {
    let idx = find_bone_idx(bones, parent.id) as usize;

    for j in 1..(bones.len() - idx) {
        if bones[idx + j].parent_id != parent.id {
            continue;
        }
        children_vec.push(bones[idx + j].clone());
        get_all_children(bones, children_vec, &bones[idx + j]);
    }
}

pub fn find_bone(bones: &Vec<Bone>, id: i32) -> Option<&Bone> {
    for b in bones {
        if b.id == id {
            return Some(&b);
        }
    }
    None
}

pub fn find_bone_idx(bones: &Vec<Bone>, id: i32) -> i32 {
    for (i, b) in bones.iter().enumerate() {
        if b.id == id {
            return i as i32;
        }
    }
    -1
}
