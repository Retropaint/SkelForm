//! Receives inputs from winit events. Most of actual input logic is handled per-module.

use crate::*;

use winit::keyboard::*;

pub fn keyboard_shortcuts(shared: &mut Shared) {
    let camera_zoom_speed = 10.;
    #[cfg(not(target_arch = "wasm32"))]
    let ui_zoom_speed = 0.01;

    if shared.input.is_pressing(KeyCode::SuperLeft) {
        // undo / redo
        if shared.input.pressed(KeyCode::KeyZ) && shared.undo_actions.len() != 0 {
            utils::undo_redo(true, shared);
        } else if shared.input.pressed(KeyCode::KeyY) && shared.redo_actions.len() != 0 {
            utils::undo_redo(false, shared);
        }

        // UI zooming
        #[cfg(not(target_arch = "wasm32"))]
        if shared.input.pressed(KeyCode::Equal) {
            shared.ui.scale += ui_zoom_speed;
        } else if shared.input.pressed(KeyCode::Minus) {
            shared.ui.scale -= ui_zoom_speed;
        }
    } else {
        // camera zooming
        if shared.input.pressed(KeyCode::Equal) {
            ui::set_zoom(shared.camera.zoom - camera_zoom_speed, shared);
        } else if shared.input.pressed(KeyCode::Minus) {
            ui::set_zoom(shared.camera.zoom + camera_zoom_speed, shared);
        }
    }

    // close all modals on esc
    if shared.input.pressed(winit::keyboard::KeyCode::Escape) {
        #[cfg(target_arch = "wasm32")]
        {
            toggleElement(false, "image-dialog".to_string());
            toggleElement(false, "file-dialog".to_string());
            toggleElement(false, "ui-slider".to_string());
        }

        shared.ui.set_state(UiState::ImageModal, false);
        shared.ui.set_state(UiState::Modal, false);
        shared.ui.set_state(UiState::PolarModal, false);
        shared.ui.set_state(UiState::ForcedModal, false);
    }

    if shared
        .input
        .is_pressing(winit::keyboard::KeyCode::SuperLeft)
    {
        // save
        if shared.input.pressed(winit::keyboard::KeyCode::KeyS) {
            #[cfg(target_arch = "wasm32")]
            utils::save_web(shared);

            #[cfg(not(target_arch = "wasm32"))]
            utils::open_save_dialog();
            //if shared.save_path == "" {
            //    utils::open_save_dialog();
            //} else {
            //    utils::save(shared.save_path.clone(), shared);
            //}
            shared.input.remove_key(&KeyCode::SuperLeft);
        }

        // open
        if shared.input.pressed(winit::keyboard::KeyCode::KeyO) {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_import_dialog(TEMP_IMPORT_PATH.to_string());
            #[cfg(target_arch = "wasm32")]
            toggleElement(true, "file-dialog".to_string());
            shared.input.remove_key(&KeyCode::SuperLeft);
        }
    }
}

pub fn mouse_wheel_input(delta: MouseScrollDelta, shared: &mut Shared) {
    if shared.input.on_ui {
        return;
    }
    let sens_reducer = 100.;
    match delta {
        MouseScrollDelta::LineDelta(_x, y) => {
            ui::set_zoom(shared.camera.zoom + (y as f32 / sens_reducer), shared);
        }

        // this is actually the touch pad
        MouseScrollDelta::PixelDelta(pos) => {
            shared.camera.pos += Vec2::new(-pos.x as f32, pos.y as f32);
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
