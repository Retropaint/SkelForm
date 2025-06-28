//! UI Armature window.

use egui::*;

use crate::{
    shared::{Shared, Vec2},
    ui as ui_mod,
};

use crate::shared::*;

pub fn draw(egui_ctx: &Context, shared: &mut Shared) {
    let min_default_size = 135.;
    let panel_id = "Armature";
    ui_mod::draw_resizable_panel(
        panel_id,
        egui::SidePanel::left(panel_id)
            .default_width(min_default_size)
            .max_width(min_default_size)
            .resizable(true)
            .show(egui_ctx, |ui| {
                ui_mod::draw_gradient(
                    ui,
                    ui.ctx().screen_rect(),
                    Color32::TRANSPARENT,
                    ui_mod::COLOR_MAIN_DARK,
                );
                ui.horizontal(|ui| {
                    ui.heading("Armature");
                });

                ui.separator();

                ui.horizontal(|ui| {
                    let button = ui_mod::button("New Bone", ui);
                    ui_mod::draw_tutorial_rect(TutorialStep::NewBone, button.rect, shared, ui);
                    if button.clicked() {
                        let idx: usize;
                        let bone: Bone;
                        if shared.selected_bone() == None {
                            (bone, idx) = shared.armature.new_bone(-1);
                        } else {
                            let id = shared.selected_bone().unwrap().id;
                            (bone, idx) = shared.armature.new_bone(id);
                        }

                        // immediately select new bone upon creating it
                        shared.ui.select_bone(idx, &shared.armature);

                        shared.undo_actions.push(Action {
                            action: ActionEnum::Bone,
                            action_type: ActionType::Created,
                            id: bone.id,
                            ..Default::default()
                        });

                        if shared.armature.bones.len() > 1 {
                            shared.ui.set_tutorial_step(TutorialStep::ReselectBone);
                        } else {
                            shared
                                .ui
                                .start_next_tutorial_step(TutorialStep::GetImage, &shared.armature);
                        }
                    }
                    let drag_name = if shared.ui.has_state(UiState::DraggingBone) {
                        "Edit"
                    } else {
                        "Drag"
                    };
                    if shared.armature.bones.len() > 1 && ui_mod::button(drag_name, ui).clicked() {
                        shared.ui.set_state(
                            UiState::DraggingBone,
                            !shared.ui.has_state(UiState::DraggingBone),
                        );
                    }
                });

                shared.ui.edit_bar_pos.x = ui.min_rect().right();

                if shared.armature.bones.len() == 0 {
                    return;
                }

                ui.add_space(3.);

                draw_hierarchy(shared, ui);
            }),
        &mut shared.input.on_ui,
        &egui_ctx,
    );
}

pub fn draw_hierarchy(shared: &mut Shared, ui: &mut egui::Ui) {
    // hierarchy
    let frame = Frame::default().inner_margin(5.);
    ui.dnd_drop_zone::<i32, _>(frame, |ui| {
        ui.set_min_width(ui.available_width());
        let mut idx = -1;

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

            ui.horizontal(|ui| {
                // add space to the left if this is a child
                {
                    let mut nb = &shared.armature.bones[b];
                    while nb.parent_id != -1 {
                        nb = shared.armature.find_bone(nb.parent_id).unwrap();
                        ui.add_space(15.);
                    }
                }

                /*
                    draggable buttons in egui don't seem well-supported, because
                    dnd_drag_source seems to physically block it. When hovering on
                    the edge of a button inside one, it will be clickable but not
                    draggable
                */

                if shared.ui.has_state(UiState::DraggingBone) {
                    // add draggable labels
                    ui.add_space(17.);
                    let id = Id::new(("bone", idx, 0));
                    let d = ui
                        .dnd_drag_source(id, idx, |ui| {
                            ui.label(RichText::new(&shared.armature.bones[b].name.to_string()));
                        })
                        .response;
                    check_bone_dragging(shared, ui, d, idx as i32);
                    return;
                }

                // show folding button, if this bone has children
                let mut children = vec![];
                get_all_children(
                    &shared.armature.bones,
                    &mut children,
                    &shared.armature.bones[b],
                );
                if children.len() > 0 {
                    let fold_icon = if shared.armature.bones[b].folded {
                        "⏵"
                    } else {
                        "⏷"
                    };
                    let rect = ui.painter().text(
                        ui.cursor().min + Vec2::new(-2., 17.).into(),
                        egui::Align2::LEFT_BOTTOM,
                        fold_icon,
                        egui::FontId::default(),
                        ui_mod::COLOR_TEXT,
                    );
                    let id = "fold".to_owned() + &shared.armature.bones[b].id.to_string();
                    let click_rect = ui
                        .interact(rect, id.into(), egui::Sense::CLICK)
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if click_rect.clicked() {
                        shared.armature.bones[b].folded = !shared.armature.bones[b].folded;
                    }
                }
                ui.add_space(13.);

                // regular, boring buttons
                let button = ui_mod::selection_button(
                    &shared.armature.bones[b].name.to_string(),
                    idx as usize == shared.ui.selected_bone_idx,
                    ui,
                );

                // highlight this bone if it's the first and is not selected during the tutorial
                if idx == 0 {
                    ui_mod::draw_tutorial_rect(TutorialStep::ReselectBone, button.rect, shared, ui);
                }

                if button.clicked() {
                    let anim_frame = shared.ui.anim.selected_frame;
                    shared.ui.select_bone(idx as usize, &shared.armature);
                    shared.ui.anim.selected_frame = anim_frame;
                };
            });
        }
    });
}

fn check_bone_dragging(shared: &mut Shared, ui: &mut egui::Ui, drag: Response, idx: i32) {
    if let (Some(pointer), Some(hovered_payload)) = (
        ui.input(|i| i.pointer.interact_pos()),
        drag.dnd_hover_payload::<i32>(),
    ) {
        let rect = drag.rect;

        let stroke = egui::Stroke::new(1.0, Color32::WHITE);
        let move_type = if *hovered_payload == idx {
            // not moved
            -1
        } else if pointer.y < rect.center().y {
            // above bone (move dragged bone above it)
            ui.painter().hline(rect.x_range(), rect.top(), stroke);
            0
        } else {
            // in bone (turn dragged bone to child)
            ui.painter().hline(rect.x_range(), rect.top(), stroke);
            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
            ui.painter().vline(rect.right(), rect.y_range(), stroke);
            ui.painter().vline(rect.left(), rect.y_range(), stroke);
            1
        };

        if let Some(dragged_payload) = drag.dnd_release_payload::<i32>() {
            // ignore if target bone is a child of this
            let mut children: Vec<Bone> = vec![];
            get_all_children(
                &shared.armature.bones,
                &mut children,
                &shared.armature.bones[*dragged_payload as usize],
            );
            for c in children {
                if shared.armature.bones[idx as usize].id == c.id {
                    return;
                }
            }

            shared.undo_actions.push(Action {
                action: ActionEnum::Bone,
                action_type: ActionType::Edited,
                id: shared.armature.bones[*dragged_payload as usize].id,
                bone: shared.armature.bones[*dragged_payload as usize].clone(),
                ..Default::default()
            });

            if move_type == 0 {
                // set dragged bone's parent as target
                shared.armature.bones[*dragged_payload as usize].parent_id =
                    shared.armature.bones[idx as usize].parent_id;
                move_bone(&mut shared.armature.bones, *dragged_payload, idx, false);
            } else if move_type == 1 {
                // move dragged bone above target
                shared.armature.bones[*dragged_payload as usize].parent_id =
                    shared.armature.bones[idx as usize].id;
                move_bone(&mut shared.armature.bones, *dragged_payload, idx, true);
            }
        }
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
