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
        Events::CamZoomIn => camera.zoom -= 10.,
        Events::CamZoomOut => camera.zoom += 10.,
        Events::CamZoomScroll => camera.zoom -= input.scroll_delta,
        Events::EditModeMove => edit_mode.current = EditModes::Move,
        Events::EditModeRotate => edit_mode.current = EditModes::Rotate,
        Events::EditModeScale => edit_mode.current = EditModes::Scale,
        Events::SelectBone => {
            selections.bone_idx = if value == -1. {
                usize::MAX
            } else {
                value as usize
            }
        }
        Events::Undo => utils::undo_redo(true, undo_states, armature, selections),
        Events::Redo => utils::undo_redo(false, undo_states, armature, selections),
        Events::OpenModal => {
            ui.modal = true;
            ui.forced_modal = if value == 1. { true } else { false };
            ui.headline = str_value;
        }
        Events::None => {}
    }
}
