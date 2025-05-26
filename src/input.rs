//! Receives inputs from winit events. Most of actual input logic is handled per-module.

use crate::*;

use winit::keyboard::*;

pub fn keyboard_shortcuts(shared: &mut Shared) {
    let camera_zoom_speed = 0.05;
    let ui_zoom_speed = 0.01;

    if shared.input.is_pressing(KeyCode::SuperLeft) {
        if shared.input.pressed(KeyCode::KeyZ) && shared.undo_actions.len() != 0 {
            utils::undo_redo(true, shared);
        } else if shared.input.pressed(KeyCode::KeyY) && shared.redo_actions.len() != 0 {
            utils::undo_redo(false, shared);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if shared.input.pressed(KeyCode::Equal) {
            shared.ui.scale += ui_zoom_speed;
        } else if shared.input.pressed(KeyCode::Minus) {
            shared.ui.scale -= ui_zoom_speed;
        }
    } else {
        if shared.input.pressed(KeyCode::Equal) {
            ui::set_zoom(shared.camera.zoom - camera_zoom_speed, shared);
        } else if shared.input.pressed(KeyCode::Minus) {
            ui::set_zoom(shared.camera.zoom + camera_zoom_speed, shared);
        }
    }

    #[cfg(target_arch = "wasm32")]
    if shared.input.pressed(winit::keyboard::KeyCode::Escape) {
        bone_panel::toggleElement(false, "image-dialog".to_string());
        bone_panel::toggleElement(false, "file-dialog".to_string());
    }

    if shared
        .input
        .is_pressing(winit::keyboard::KeyCode::SuperLeft)
    {
        if shared.input.pressed(winit::keyboard::KeyCode::KeyS) {
            #[cfg(target_arch = "wasm32")]
            utils::save_web(shared);

            #[cfg(not(target_arch = "wasm32"))]
            if shared.save_path == "" {
                utils::open_save_dialog();
            } else {
                utils::save(shared.save_path.clone(), shared);
            }
        }
        if shared.input.pressed(winit::keyboard::KeyCode::KeyO) {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_import_dialog();
            #[cfg(target_arch = "wasm32")]
            bone_panel::toggleElement(true, "file-dialog".to_string());
        }
    }
}

pub fn mouse_wheel_input(delta: MouseScrollDelta, shared: &mut Shared) {
    let sens_reducer = 100.;
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            ui::set_zoom(shared.camera.zoom + (y as f32 / sens_reducer), shared);
        }

        // this is actually the touch pad
        MouseScrollDelta::PixelDelta(pos) => {
            shared.camera.pos += Vec2::new(-pos.x as f32, pos.y as f32) / sens_reducer;
        }
    }
}

pub fn pinch(delta: f64, shared: &mut Shared) {
    let sens_amp = 4.;
    shared.camera.zoom -= delta as f32 * sens_amp;
}
