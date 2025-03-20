//! Isolated set of helper functions

use crate::Vec2;

/// Convert a point from screen to world space
pub fn screen_to_world_space(pos: Vec2, window: Vec2) -> Vec2 {
    Vec2{
        x: -1. + ((pos.x / window.x as f32) * 2.),
        y: -(-1. + ((pos.y / window.y as f32) * 2.)),
    }
}
