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
            .min_width(min_default_size)
            .max_width(min_default_size)
            .resizable(true)
            .show(egui_ctx, |ui| {
                ui_mod::draw_gradient(
                    ui,
                    ui.ctx().screen_rect(),
                    Color32::TRANSPARENT,
                    shared.config.ui_colors.gradient.into(),
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

                        shared.undo_actions.push(Action {
                            action: ActionEnum::Bones,
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
                        shared.ui.select_bone(idx, &shared.armature);

                        if shared.armature.bones.len() > 1 {
                            shared.ui.set_tutorial_step(TutorialStep::ReselectBone);
                        } else {
                            shared
                                .ui
                                .start_next_tutorial_step(TutorialStep::GetImage, &shared.armature);
                        }
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
        let mut idx: i32 = -1;

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

            let mut dragged = false;

            ui.horizontal(|ui| {
                // add space to the left if this is a child
                let mut nb = &shared.armature.bones[b];
                while nb.parent_id != -1 {
                    nb = shared.armature.find_bone(nb.parent_id).unwrap();
                    ui.add_space(15.);
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
                        shared.config.ui_colors.text.into(),
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

                let mut selected_col = shared.config.ui_colors.light_accent;
                let mut cursor = egui::CursorIcon::PointingHand;

                if shared.ui.selected_bone_idx == idx as usize {
                    selected_col += Color::new(20, 20, 20, 0);
                    cursor = egui::CursorIcon::Default;
                }

                let id = Id::new(("bone", idx, 0));
                let button = ui
                    .dnd_drag_source(id, idx, |ui| {
                        ui.add(
                            egui::Button::new(&shared.armature.bones[b].name.to_string())
                                .fill(selected_col),
                        )
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_cursor(cursor);

                if button.clicked() {
                    let anim_frame = shared.ui.anim.selected_frame;
                    shared.ui.select_bone(idx as usize, &shared.armature);
                    shared.ui.anim.selected_frame = anim_frame;
                };

                if button.secondary_clicked() {
                    shared.ui.context_menu.show(ContextType::Bone, idx as i32);
                }

                if shared.ui.context_menu.is(ContextType::Bone, idx as i32) {
                    button.show_tooltip_ui(|ui| {
                        if crate::ui::clickable_label(ui, "Delete").clicked() {
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

                // highlight this bone if it's the first and is not selected during the tutorial
                if idx == 0 {
                    ui_mod::draw_tutorial_rect(TutorialStep::ReselectBone, button.rect, shared, ui);
                }

                if check_bone_dragging(shared, ui, button, idx as usize) {
                    dragged = true;
                }
            });

            if dragged {
                return;
            }
        }
    });
}

fn check_bone_dragging(shared: &mut Shared, ui: &mut egui::Ui, drag: Response, idx: usize) -> bool {
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let hovered_payload = drag.dnd_hover_payload::<i32>();
    let rect = drag.rect;
    let stroke = egui::Stroke::new(1.0, Color32::WHITE);

    if pointer == None || hovered_payload == None {
        // render hover box
        let stroke = egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(125, 125, 125, 255));
        if drag.contains_pointer() {
            ui.painter().hline(rect.x_range(), rect.top(), stroke);
            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
            ui.painter().vline(rect.right(), rect.y_range(), stroke);
            ui.painter().vline(rect.left(), rect.y_range(), stroke);
        }
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

    let dp = drag.dnd_release_payload::<i32>();
    if dp == None {
        return false;
    }
    let dragged_payload = *dp.unwrap() as usize;

    let dragged_id = shared.armature.bones[dragged_payload as usize].id;
    let mut selected_id: i32 = 0;
    if shared.selected_bone() != None {
        selected_id = shared.selected_bone().unwrap().id;
    }

    // ignore if target bone is a child of this
    let mut children: Vec<Bone> = vec![];
    get_all_children(
        &shared.armature.bones,
        &mut children,
        &shared.armature.bones[dragged_payload as usize],
    );
    for c in children {
        if shared.armature.bones[idx as usize].id == c.id {
            return false;
        }
    }

    shared.undo_actions.push(Action {
        action: ActionEnum::Bones,
        bones: shared.armature.bones.clone(),
        ..Default::default()
    });

    let old_parents = shared
        .armature
        .get_all_parents(shared.armature.bones[dragged_payload].id);

    drag_bone(shared, is_above, dragged_payload, idx);

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
