//! Receives inputs from winit events. Most of actual input logic is handled per-module.

use crate::*;

pub fn mouse_wheel_input(delta: MouseScrollDelta, shared: &mut Shared) {
    if shared.input.on_ui {
        return;
    }
    let _sens_reducer = 100.;
    match delta {
        MouseScrollDelta::LineDelta(_x, _y) => {
            // ui::set_zoom(shared.camera.zoom + (y as f32 / sens_reducer), shared);
        }

        // this is actually the touch pad
        MouseScrollDelta::PixelDelta(_pos) => {
            // shared.camera.pos += Vec2::new(-pos.x as f32, pos.y as f32);
        }
    }
}

pub fn pinch(delta: f64, shared: &mut Shared) {
    if shared.input.on_ui {
        return;
    }
    let sens_amp = 500.;
    shared.camera.zoom -= delta as f32 * sens_amp;
}
