//! UI Armature window.

use egui::*;

use crate::{
    shared::{Shared, Vec2},
    ui as ui_mod,
};

use crate::shared::*;

pub fn draw(egui_ctx: &Context, shared: &mut Shared) {
    let min_default_size = 135. * ui_mod::FONT_SCALE;
    let response = egui::SidePanel::left("Armature")
        .default_width(min_default_size)
        .min_width(min_default_size)
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
                if shared.tutorial_step == TutorialStep::NewBone {
                    ui_mod::draw_fading_rect(ui, button.rect, Color32::GOLD, 60., 1.);
                }
                if button.clicked() {
                    let idx: usize;
                    let bone: Bone;
                    if shared.selected_bone() == None {
                        (bone, idx) = new_bone(shared, -1);
                    } else {
                        (bone, idx) = new_bone(shared, shared.selected_bone().unwrap().id);
                    }

                    // immediately select new bone upon creating it
                    shared.select_bone(idx);

                    shared.undo_actions.push(Action {
                        action: ActionEnum::Bone,
                        action_type: ActionType::Created,
                        id: bone.id,
                        ..Default::default()
                    });

                    shared.next_tutorial_step();
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

            // hierarchy
            let frame = Frame::default().inner_margin(5.);
            ui.dnd_drop_zone::<i32, _>(frame, |ui| {
                ui.set_min_width(ui.available_width());
                let mut idx = 0;
                for s in shared.armature.bones.clone() {
                    ui.horizontal(|ui| {
                        // add space to the left if this is a child
                        let mut nb: &Bone = &s;
                        while nb.parent_id != -1 {
                            nb = find_bone(&shared.armature.bones, nb.parent_id).unwrap();
                            ui.add_space(20.);
                        }

                        /*
                            draggable buttons in egui don't seem well-supported, because
                            dnd_drag_source seems to physically block it. When hovering on
                            the edge of a button inside one, it will be clickable but not
                            draggable
                        */

                        if shared.ui.has_state(UiState::DraggingBone) {
                            // add draggable labels
                            ui.add_space(4.);
                            let id = Id::new(("bone", idx, 0));
                            let d = ui
                                .dnd_drag_source(id, idx, |ui| {
                                    ui.label(RichText::new(&s.name.to_string()));
                                })
                                .response;
                            check_bone_dragging(shared, ui, d, idx as i32);
                        } else {
                            // regular, boring buttons

                            if ui_mod::selection_button(
                                &s.name.to_string(),
                                idx as usize == shared.selected_bone_idx,
                                ui,
                            )
                            .clicked()
                            {
                                shared.select_bone(idx as usize);
                            };
                        }
                        idx += 1;
                    });
                }
            });
        })
        .response;
    if response.hovered() {
        shared.input.on_ui = true;
    }
}

pub fn new_bone(shared: &mut Shared, id: i32) -> (Bone, usize) {
    let mut parent_id = -1;
    if shared.find_bone(id) != None {
        parent_id = shared.find_bone(id).unwrap().parent_id;
    }
    let new_bone = Bone {
        name: NEW_BONE_NAME.to_string(),
        parent_id,
        id: generate_id(&shared.armature.bones),
        scale: Vec2 { x: 1., y: 1. },
        tex_idx: -1,
        pivot: Vec2::new(0.5, 0.5),
        zindex: shared.armature.bones.len() as f32,
        ..Default::default()
    };
    if id == -1 {
        shared.armature.bones.push(new_bone.clone());
    } else {
        // add new bone below targeted one, keeping in mind its children
        for i in 0..shared.armature.bones.len() {
            if shared.armature.bones[i].id != id {
                continue;
            }

            let mut children = vec![];
            crate::armature_window::get_all_children(
                &shared.armature.bones,
                &mut children,
                &shared.armature.bones[i],
            );
            let idx = i + children.len() + 1;
            shared.armature.bones.insert(idx, new_bone.clone());
            return (new_bone, idx);
        }
    }
    (new_bone, shared.armature.bones.len() - 1)
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
    let mut i: usize = 1;
    let idx = find_bone_idx(bones, parent.id);

    #[rustfmt::skip]
    macro_rules! check_bounds {() => {
        if idx as usize + i > bones.len() - 1 {
            return;
        }};
    }

    check_bounds!();

    while bones[idx as usize + i].parent_id == parent.id {
        let prev_len = children_vec.len();
        children_vec.push(bones[idx as usize + i].clone());
        get_all_children(bones, children_vec, &bones[idx as usize + i]);

        // move to the next bone that is not a child of this one
        i += children_vec.len() - prev_len;

        check_bounds!();
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

// generate non-clashing id
pub fn generate_id(bones: &Vec<Bone>) -> i32 {
    let mut idx = 0;
    while idx == does_id_exist(bones, idx) {
        idx += 1;
    }
    return idx;
}

fn does_id_exist(bones: &Vec<Bone>, id: i32) -> i32 {
    for b in bones {
        if b.id == id {
            return id;
        }
    }
    return -1;
}
