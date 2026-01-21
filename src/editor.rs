use crate::*;

pub fn iterate_events(
    events: &mut EventState,
    camera: &mut Camera,
    input: &InputStates,
    edit_mode: &mut EditMode,
    selections: &mut SelectionState,
    undo_states: &mut UndoStates,
    armature: &mut Armature,
    ui: &mut crate::Ui,
) {
    let mut last_event = Events::None;
    while events.events.len() > 0 {
        // for every new event, create a new undo state
        if last_event != events.events[0] {
            last_event = events.events[0].clone();
            match last_event {
                Events::DragBone => undo_states.new_undo_bones(&armature.bones),
                _ => {}
            }
        }

        if events.events[0] == Events::DragBone {
            // dropping dragged bone and moving it (or setting it as child)

            let old_parents = armature.get_all_parents(events.values[1] as i32);

            #[rustfmt::skip] macro_rules! dragged {()=>{armature.find_bone_mut(events.values[1] as i32).unwrap()}}
            #[rustfmt::skip] macro_rules! pointing{()=>{armature.find_bone_mut(events.values[0] as i32).unwrap()}}
            #[rustfmt::skip] macro_rules! bones   {()=>{&mut armature.bones}}

            #[rustfmt::skip] let drag_idx = bones!().iter().position(|b| b.id == events.values[1] as i32).unwrap() as i32;
            #[rustfmt::skip] let point_idx = bones!().iter().position(|b| b.id == events.values[0] as i32).unwrap() as i32;

            if events.values[2] == 1. {
                // set pointed bone's parent as dragged bone's parent
                dragged!().parent_id = pointing!().parent_id;
                move_bone(bones!(), drag_idx, point_idx, false);
            } else {
                // set pointed bone as dragged bone's parent
                dragged!().parent_id = pointing!().id;
                move_bone(bones!(), drag_idx, point_idx, true);
                pointing!().folded = false;
            }

            // keep bone selected in new dragged position
            let bones = &mut armature.bones;
            let sel_bone = events.values[1] as i32;
            let bone_idx = bones.iter().position(|b| b.id == sel_bone).unwrap();
            selections.bone_ids = vec![events.values[1] as i32];
            selections.bone_idx = bone_idx;

            // adjust dragged bone so it stays in place
            armature.offset_pos_by_parent(old_parents, events.values[1] as i32);

            events.events.remove(0);
            events.values.drain(0..=2);
        } else {
            // normal events: 1 event ID, 1 set of value(s)

            let event = &events.events[0].clone();
            let value = events.values[0];
            let str_value = events.str_values[0].clone().to_string();

            #[rustfmt::skip]
            editor::process_event(event, value, str_value, camera, &input, edit_mode, selections, undo_states, armature, ui);

            events.events.remove(0);
            events.values.remove(0);
            events.str_values.remove(0);
        }
    }
}

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
        Events::CamZoomIn => camera.zoom -= 10.,
        Events::CamZoomOut => camera.zoom += 10.,
        Events::CamZoomScroll => camera.zoom -= input.scroll_delta,
        Events::EditModeMove => edit_mode.current = EditModes::Move,
        Events::EditModeRotate => edit_mode.current = EditModes::Rotate,
        Events::EditModeScale => edit_mode.current = EditModes::Scale,
        Events::UnselectAll => unselect_all(selections, ui),
        Events::Undo => undo_redo(true, undo_states, armature, selections),
        Events::Redo => undo_redo(false, undo_states, armature, selections),
        Events::SelectBone => {
            selections.bone_idx = if value == f32::MAX {
                usize::MAX
            } else {
                value as usize
            };
            selections.bone_ids = vec![armature.bones[value as usize].id];
        }
        Events::OpenModal => {
            ui.modal = true;
            ui.forced_modal = value == 1.;
            ui.headline = str_value;
        }
        Events::SelectAnimFrame => {
            let selected_anim = selections.anim;
            unselect_all(selections, ui);
            selections.anim = selected_anim;
            selections.anim_frame = value as i32;
        }
        Events::OpenPolarModal => {
            ui.polar_id = match value {
                0. => PolarId::DeleteBone,
                1. => PolarId::Exiting,
                2. => PolarId::DeleteAnim,
                3. => PolarId::DeleteFile,
                4. => PolarId::DeleteTex,
                5. => PolarId::DeleteStyle,
                6. => PolarId::NewUpdate,
                _ => return,
            };
            ui.polar_modal = true;
            ui.headline = str_value.to_string();
        }
        Events::PointerOnUi => {
            camera.on_ui = value == 1.;
        }
        _ => {}
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

pub fn move_bone(bones: &mut Vec<Bone>, old_idx: i32, new_idx: i32, is_setting_parent: bool) {
    let main = &bones[old_idx as usize];
    let anchor = bones[new_idx as usize].clone();

    // gather all bones to be moved (this and its children)
    let mut to_move: Vec<Bone> = vec![main.clone()];
    armature_window::get_all_children(bones, &mut to_move, main);

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
            armature_window::find_bone_idx(bones, anchor.id) as usize + is_setting_parent as usize,
            bone.clone(),
        );
    }
}
