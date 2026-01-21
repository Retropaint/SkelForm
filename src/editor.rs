use crate::*;

pub fn process_event(
    event: &crate::Events,
    value: f32,
    str_value: String,
    camera: &mut Camera,
    input: &InputStates,
    edit_mode: &mut EditMode,
    selections: &mut SelectionState,
    undo_states: &mut UndoStates,
    armature: &mut Armature,
    ui: &mut crate::Ui,
) {
    match event {
        Events::None => {}
        Events::CamZoomIn => camera.zoom -= 10.,
        Events::CamZoomOut => camera.zoom += 10.,
        Events::CamZoomScroll => camera.zoom -= input.scroll_delta,
        Events::EditModeMove => edit_mode.current = EditModes::Move,
        Events::EditModeRotate => edit_mode.current = EditModes::Rotate,
        Events::EditModeScale => edit_mode.current = EditModes::Scale,
        Events::SelectBone => {
            selections.bone_idx = if value == f32::MAX {
                usize::MAX
            } else {
                value as usize
            }
        }
        Events::Undo => undo_redo(true, undo_states, armature, selections),
        Events::Redo => undo_redo(false, undo_states, armature, selections),
        Events::OpenModal => {
            ui.modal = true;
            ui.forced_modal = value == 1.;
            ui.headline = str_value;
        }
        Events::UnselectAll => unselect_all(selections, ui),
        Events::SelectAnimFrame => {
            let selected_anim = selections.anim;
            unselect_all(selections, ui);
            selections.anim = selected_anim;
            selections.anim_frame = value as i32;
        }
    }
}

fn unselect_all(selections: &mut SelectionState, ui: &mut crate::Ui) {
    selections.bone_idx = usize::MAX;
    selections.bone_ids = vec![];
    selections.anim_frame = -1;
    selections.anim = usize::MAX;
    selections.bind = -1;
    ui.showing_mesh = false;
}

pub fn undo_redo(
    undo: bool,
    undo_states: &mut UndoStates,
    armature: &mut Armature,
    selections: &mut SelectionState,
) {
    let action: Action;
    if undo {
        if undo_states.undo_actions.last() == None {
            return;
        }
        action = undo_states.undo_actions.last().unwrap().clone();
    } else {
        if undo_states.redo_actions.last() == None {
            return;
        }
        action = undo_states.redo_actions.last().unwrap().clone();
    }

    // store the state prior to undoing/redoing the action,
    // to add to the opposite stack later
    let mut new_action = action.clone();

    match &action.action {
        ActionType::Bone => {
            new_action.bones = armature.bones.clone();
            *armature.find_bone_mut(action.bones[0].id).unwrap() = action.bones[0].clone();
        }
        ActionType::Bones => {
            new_action.bones = armature.bones.clone();
            armature.bones = action.bones.clone();
            if selections.bone_ids.len() == 0 {
                selections.bone_idx = usize::MAX;
            } else {
                let sel_id = selections.bone_ids[0];
                let sel_idx = armature.bones.iter().position(|b| b.id == sel_id);
                if sel_idx != None {
                    selections.bone_idx = sel_idx.unwrap();
                } else {
                    selections.bone_idx = usize::MAX
                }
            }
        }
        ActionType::Animation => {
            new_action.animations = armature.animations.clone();
            let anims = &mut armature.animations;
            let anim = anims.iter_mut().find(|a| a.id == action.animations[0].id);
            *anim.unwrap() = action.animations[0].clone();
        }
        ActionType::Animations => {
            new_action.animations = armature.animations.clone();
            armature.animations = action.animations.clone();
            let animations = &mut armature.animations;
            if animations.len() == 0 || selections.anim > animations.len() - 1 {
                selections.anim = usize::MAX;
            }
        }
        ActionType::Style => {
            new_action.styles = armature.styles.clone();
            let styles = &mut armature.styles;
            let id = action.styles[0].id;
            let style = styles.iter_mut().find(|a| a.id == id).unwrap();
            *style = action.styles[0].clone();
        }
        ActionType::Styles => {
            new_action.styles = armature.styles.clone();
            armature.styles = action.styles.clone();
            let style_ids: Vec<i32> = armature.styles.iter().map(|s| s.id).collect();
            if !style_ids.contains(&selections.style) {
                selections.style = -1;
            }
        }
        _ => {}
    }

    // add action(s) to opposing stack
    undo_states.temp_actions.push(new_action);
    if undo {
        undo_states.undo_actions.pop();
        if !action.continued {
            // reverse list to restore order of actions
            undo_states.temp_actions.reverse();
            undo_states
                .redo_actions
                .append(&mut undo_states.temp_actions);
            undo_states.temp_actions = vec![];
        }
    } else {
        undo_states.redo_actions.pop();
        if !action.continued {
            // ditto
            undo_states.temp_actions.reverse();
            undo_states
                .undo_actions
                .append(&mut undo_states.temp_actions);
            undo_states.temp_actions = vec![];
        }
    }

    // actions tagged with `continue` are part of an action chain
    if action.continued {
        undo_redo(undo, undo_states, armature, selections);
    }
}
