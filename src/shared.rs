//! Easily-accessible and frequently-shared data

use wgpu::BindGroup;

use crate::Vec2;

#[derive(Clone, Default)]
pub struct BoneTexture {
    pub idx: usize, // index relative to skelements texture vector
}

#[derive(Clone, Default)]
pub struct Bone {
    pub name: String,
    pub parent_id: i32,
    pub pos: Vec2,
    pub rot: f32,
    pub scale: Vec2,
    pub id: i32,
    pub tex: BoneTexture,

    // used to properly offset bone's movement to counteract it's parent
    pub parent_rot: f32,
}

#[derive(Clone, Default)]
pub struct Armature {
    pub bones: Vec<Bone>,
}

#[derive(Default)]
pub struct Shared {
    pub mouse: Vec2,
    pub window: Vec2,
    pub bind_groups: Vec<BindGroup>,
    pub dragging: bool,
    pub selected_bone: usize,
    pub armature: Armature,
}
