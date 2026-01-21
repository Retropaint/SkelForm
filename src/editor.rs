use crate::*;

pub fn process_events(
    events: &mut crate::EventState,
    camera: &mut Camera,
    input: &InputStates,
    edit_mode: &mut EditMode,
) {
    while events.events.len() > 0 {
        let event = events.events.last().unwrap();
        match event {
            Events::CamZoomIn => camera.zoom -= 10.,
            Events::CamZoomOut => camera.zoom += 10.,
            Events::CamZoomScroll => camera.zoom -= input.scroll_delta,
            Events::EditModeMove => *edit_mode = EditMode::Move,
            Events::EditModeRotate => *edit_mode = EditMode::Rotate,
            Events::EditModeScale => *edit_mode = EditMode::Scale,
            Events::None => {}
        }
        events.events.pop();
    }
}
