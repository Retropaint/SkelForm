//! UI Armature window.

use egui::*;

use crate::{
    shared::{Shared, Vec2},
    ui,
    ui::EguiUi,
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
                ui.heading(shared.loc("armature_panel.heading"));
            });

            ui.separator();

            ui.horizontal(|ui| {
                let button = ui.skf_button(shared.loc("armature_panel.new_bone_button"));
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
                        shared.loc("armature_panel.new_bone_name").to_string();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if shared.armature.bones.len() == 0 {
                        return;
                    }
                    let mut selected_style = -1;
                    let dropdown = egui::ComboBox::new("styles", "")
                        .selected_text(shared.loc("armature_panel.styles"))
                        .width(80.)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show_ui(ui, |ui| {
                            for s in 0..shared.armature.styles.len() {
                                ui.set_width(80.);
                                let tick = if shared.armature.styles[s].active {
                                    " ✅"
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
                        .on_hover_text(shared.loc("armature_panel.styles_desc"));

                    if shared.ui.has_state(UiState::FocusStyleDropdown) {
                        dropdown.request_focus();
                        shared.ui.set_state(UiState::FocusStyleDropdown, false);
                    }
                    if selected_style == -2 {
                        shared.open_style_modal();
                    } else if selected_style != -1 {
                        shared.ui.selected_style = selected_style;
                        for b in 0..shared.armature.bones.len() {
                            if shared.armature.bones[b]
                                .style_idxs
                                .contains(&selected_style)
                            {
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

            if shared.armature.bones.len() == 0 {
                return;
            }

            ui.add_space(3.);

            egui::ScrollArea::both()
                .max_height(ui.available_height() - 10.)
                .show(ui, |ui| {
                    // hierarchy
                    let frame = Frame::default().inner_margin(5.);
                    ui.dnd_drop_zone::<i32, _>(frame, |ui| {
                        draw_hierarchy(shared, ui);
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

                if shared.ui.selected_bone_idx == idx as usize {
                    selected_col += Color::new(20, 20, 20, 0);
                    cursor = egui::CursorIcon::Default;
                }

                let width = ui.available_width();

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

                                //let pic = if has_tex { "🖻  " } else { "" };
                                let pic = "";
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
                    if shared.ui.setting_ik_target {
                        shared.selected_bone_mut().unwrap().ik_target_id = bone_id;
                        shared.ui.setting_ik_target = false;
                    } else {
                        let anim_frame = shared.ui.anim.selected_frame;
                        shared.ui.select_bone(idx as usize);
                        shared.ui.anim.selected_frame = anim_frame;
                    }
                };

                if button.secondary_clicked() {
                    shared
                        .ui
                        .context_menu
                        .show(ContextType::Bone, shared.armature.bones[b].id);
                }

                if shared
                    .ui
                    .context_menu
                    .is(ContextType::Bone, shared.armature.bones[b].id)
                {
                    button.show_tooltip_ui(|ui| {
                        if ui.clickable_label("Delete").clicked() {
                            shared.ui.open_polar_modal(
                                PolarId::DeleteBone,
                                "Are you sure to delete this bone?",
                            );
                            shared.ui.context_menu.hide = true;
                        };

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

    // prevent one from being draggable onto itself
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

    let dp = if let Some(dp) = drag.dnd_release_payload::<i32>() {
        *dp as usize
    } else {
        return false;
    };

    let dragged_id = shared.armature.bones[dp].id;

    let selected_id = shared.selected_bone_id();

    // ignore if target bone is a child of this
    let mut children: Vec<Bone> = vec![];
    get_all_children(
        &shared.armature.bones,
        &mut children,
        &shared.armature.bones[dp],
    );
    for c in children {
        if shared.armature.bones[idx as usize].id == c.id {
            return false;
        }
    }

    shared.undo_actions.push(Action {
        action: ActionType::Bones,
        bones: shared.armature.bones.clone(),
        ..Default::default()
    });

    let old_parents = shared
        .armature
        .get_all_parents(shared.armature.bones[dp].id);

    drag_bone(shared, is_above, dp, idx);

    if shared.selected_bone() != None {
        shared.ui.selected_bone_idx = shared.armature.find_bone_idx(selected_id).unwrap();
    }

    shared
        .armature
        .offset_bone_by_parent(old_parents, dragged_id);

    return true;
}

pub fn drag_bone(shared: &mut Shared, is_above: bool, dragged_idx: usize, pointing_idx: usize) {
    if is_above {
        // set pointed bone's parent as dragged bone's parent
        shared.armature.bones[dragged_idx].parent_id =
            shared.armature.bones[pointing_idx].parent_id;
        move_bone(
            &mut shared.armature.bones,
            dragged_idx as i32,
            pointing_idx as i32,
            false,
        );
    } else {
        // set pointed bone as dragged bone's parent
        let parent_id = shared.armature.bones[pointing_idx].id;
        shared.armature.bones[dragged_idx].parent_id = parent_id;
        move_bone(
            &mut shared.armature.bones,
            dragged_idx as i32,
            pointing_idx as i32,
            true,
        );

        shared.armature.find_bone_mut(parent_id).unwrap().folded = false;
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
    for b in &to_move {
        bones.insert(
            find_bone_idx(bones, anchor.id) as usize + is_setting_parent as usize,
            b.clone(),
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
