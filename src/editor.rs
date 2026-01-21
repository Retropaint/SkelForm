use crate::*;

pub fn process(events: &mut Vec<crate::Event>, camera: &mut Camera) {
    while events.len() > 0 {
        let event = events.last().unwrap();
        match event.id {
            Events::CamZoomIn => camera.zoom -= 10.,
            Events::CamZoomOut => camera.zoom += 10.,
            Events::None => {}
        }
        events.pop();
    }
}
