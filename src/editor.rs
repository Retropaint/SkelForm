use crate::*;

pub fn process(events: &mut Vec<crate::Event>, camera: &mut Camera, input: &InputStates) {
    while events.len() > 0 {
        let event = events.last().unwrap();
        match event.id {
            Events::CamZoomIn => camera.zoom -= 10.,
            Events::CamZoomOut => camera.zoom += 10.,
            Events::CamZoomScroll => camera.zoom -= input.scroll_delta,
            Events::None => {}
        }
        events.pop();
    }
}
