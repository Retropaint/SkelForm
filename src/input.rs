//! Receives inputs from winit events. Most of actual input logic is handled per-module.

use crate::*;

use winit::event::ElementState;
use winit::keyboard::*;

pub fn keyboard_shortcuts(shared: &mut Shared) {
    let camera_zoom_speed = 0.05;
    let ui_zoom_speed = 0.01;

    let mut undo = false;
    let mut redo = false;
    if shared.input.is_pressing(KeyCode::SuperLeft) {
        if shared.input.pressed(KeyCode::KeyZ) && shared.undo_actions.len() != 0 {
            undo = true;
        } else if shared.input.pressed(KeyCode::KeyY) && shared.redo_actions.len() != 0 {
            redo = true;
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

    if undo || redo {
        let action: Action;
        if undo {
            action = shared.undo_actions.last().unwrap().clone();
        } else {
            action = shared.redo_actions.last().unwrap().clone();
        }
        let mut new_action = action.clone();

        match &action.action {
            ActionEnum::Bone => {
                if action.action_type == ActionType::Created {
                    shared.selected_bone_idx = usize::MAX;
                    if undo {
                        for (i, bone) in shared.armature.bones.iter().enumerate() {
                            if bone.id == action.id {
                                shared.armature.bones.remove(i);
                                break;
                            }
                        }
                    } else {
                        armature_window::new_bone(shared, -1);
                    }
                } else {
                    new_action.bone = shared.armature.bones[action.id as usize].clone();
                    *shared.find_bone_mut(action.id).unwrap() = action.bone.clone();

                    for i in 0..shared.armature.bones.len() {
                        shared.organize_bone(i);
                    }
                }
            }
            ActionEnum::Animation => {
                if action.action_type == ActionType::Created {
                    shared.ui.anim.selected = usize::MAX;
                    if undo {
                        shared.armature.animations.pop();
                    } else {
                        keyframe_editor::new_animation(shared);
                    }
                } else {
                    new_action.animation = shared.armature.animations[action.id as usize].clone();
                    shared.armature.animations[action.id as usize] = action.animation.clone();
                }
            }
            _ => {}
        }

        if undo {
            shared.redo_actions.push(new_action);
            shared.undo_actions.pop();
        } else {
            shared.undo_actions.push(new_action);
            shared.redo_actions.pop();
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
