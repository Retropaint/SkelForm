//! Struct for easily-accessible, frequently-shared data.

use wgpu::BindGroup;

use crate::Vec2;

#[derive(Default)]
pub struct Shared {
    pub mouse: Vec2,
    pub window: Vec2,
    pub bind_groups: Vec<BindGroup>,
}
