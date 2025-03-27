//! Receives inputs from winit events. Actual input logic is handled per-module.

use crate::*;

use winit::event::ElementState;

pub fn keyboard_input(
    key: &winit::keyboard::KeyCode,
    state: &winit::event::ElementState,
    shared: &mut crate::shared::Shared,
) {
    if *key == winit::keyboard::KeyCode::KeyW {
        shared.armature.bones[1].tex_idx = 0;
        shared.armature.bones[2].tex_idx = 0;
    }

    if *key == winit::keyboard::KeyCode::SuperLeft {
        if *state == ElementState::Pressed {
            shared.input.modifier = 1;
        } else {
            shared.input.modifier = -1;
        }
    }
}

pub fn mouse_input(button: &crate::MouseButton, state: &ElementState, shared: &mut crate::shared::Shared) {
    if *button == MouseButton::Left {
        if *state == ElementState::Pressed {
            shared.input.mouse_left = 0;
        } else {
            shared.input.mouse_left = -1;
            shared.input.mouse_bone_offset = None;
        }
    }
}
