//! Easily-accessible and frequently-shared data

use wgpu::BindGroup;

use crate::Vec2;

#[derive(Clone, Default)]
pub struct Bone {
    pub id: i32,
    pub name: String,
    pub parent_id: i32,
    pub pos: Vec2,
    pub rot: f32,
    pub scale: Vec2,
    pub tex_idx: usize,

    /// used to properly offset bone's movement to counteract it's parent
    pub parent_rot: f32,
}

#[derive(Clone, Default)]
pub struct Armature {
    /// index relative to skelements texture vector
    pub bones: Vec<Bone>,

    pub textures: Vec<Texture>,
}

#[derive(Clone, Default)]
pub struct Texture {
    pub size: Vec2,
    pub pixels: Vec<u8>,
}

#[derive(Default)]
pub struct Shared {
    pub mouse: Vec2,
    pub window: Vec2,
    pub dragging: bool,
    pub selected_bone: usize,
    pub armature: Armature,
    pub bind_groups: Vec<BindGroup>,

    pub mouse_bone_offset: Option<Vec2>,

    /// how long the left click has been held for
    pub mouse_left: i32,

    /// useful if you don't want to provide an actual bind group during testing
    pub placeholder_bind_group: Option<BindGroup>,
}
