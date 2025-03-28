//! Receives inputs from winit events. Most of actual input logic is handled per-module.

use crate::*;

use winit::event::ElementState;
use winit::keyboard::*;

pub fn keyboard_input(
    key: &winit::keyboard::KeyCode,
    state: &winit::event::ElementState,
    shared: &mut crate::shared::Shared,
) {
    if *key == KeyCode::KeyW {
        shared.armature.bones[1].tex_idx = 0;
        shared.armature.bones[2].tex_idx = 0;
    }

    // Record all pressed keys (and remove released ones)
    if *state == ElementState::Pressed {
        let mut add = true;
        for pressed_key in &mut shared.input.pressed {
            if key == pressed_key {
                add = false;
                break;
            }
        }
        if add {
            shared.input.pressed.push(*key);
        }
    } else {
        for (i, pressed_key) in &mut shared.input.pressed.iter().enumerate() {
            if pressed_key == key {
                shared.input.pressed.remove(i);
                break;
            }
        }
    }

    if is_pressing(KeyCode::Equal, &shared) {
        ui::set_zoom(shared.zoom - 0.1, shared)
    } else if is_pressing(KeyCode::Minus, &shared) {
        ui::set_zoom(shared.zoom + 0.1, shared)
    }

    if shared.input.modifier != -1 {
        // move camera if holding mod key
        if let Some(im) = shared.input.initial_mouse {
            let mouse_world = utils::screen_to_world_space(shared.input.mouse, shared.window);
            let initial_world = utils::screen_to_world_space(im, shared.window);
            shared.camera.pos = shared.camera.initial_pos - (mouse_world - initial_world);
        } else {
            shared.camera.initial_pos = shared.camera.pos;
            shared.input.initial_mouse = Some(shared.input.mouse);
        }
    }

    if *key == KeyCode::SuperLeft {
        if *state == ElementState::Pressed {
            shared.input.modifier = 1;
        } else {
            shared.input.modifier = -1;
        }
    }
}

pub fn mouse_input(
    button: &crate::MouseButton,
    state: &ElementState,
    shared: &mut crate::shared::Shared,
) {
    if *button == MouseButton::Left {
        if *state == ElementState::Pressed {
            shared.input.mouse_left = 0;
        } else {
            shared.input.mouse_left = -1;
            shared.input.mouse_bone_offset = None;
        }
    }

    if shared.input.mouse_left == -1 {
        shared.input.initial_mouse = None;
    } else {
        // move camera if holding mod key
        if let Some(im) = shared.input.initial_mouse {
            let mouse_world = utils::screen_to_world_space(shared.input.mouse, shared.window);
            let initial_world = utils::screen_to_world_space(im, shared.window);
            shared.camera.pos = shared.camera.initial_pos - (mouse_world - initial_world);
        } else {
            shared.camera.initial_pos = shared.camera.pos;
            shared.input.initial_mouse = Some(shared.input.mouse);
        }
    }
}

pub fn is_pressing(key: KeyCode, shared: &Shared) -> bool {
    for k in &shared.input.pressed {
        if *k == key {
            return true;
        }
    }
    false
}
