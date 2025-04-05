//! Easily-accessible and frequently-shared data.

use std::{
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use egui::{text::CCursor, Context};
use wgpu::BindGroup;
use winit::{keyboard::KeyCode, window::CursorIcon};

#[repr(C)]
#[derive(serde::Serialize, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl From<egui::Pos2> for Vec2 {
    fn from(pos: egui::Pos2) -> Vec2 {
        Vec2::new(pos.x, pos.y)
    }
}

impl From<egui::Vec2> for Vec2 {
    fn from(pos: egui::Vec2) -> Vec2 {
        Vec2::new(pos.x, pos.y)
    }
}

impl Into<egui::Pos2> for Vec2 {
    fn into(self) -> egui::Pos2 {
        egui::Pos2::new(self.x, self.y)
    }
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0., 0.);

    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn equal_to(self: &Self, other: Vec2) -> bool {
        return self.x != other.x || self.y != other.y;
    }
}

impl MulAssign for Vec2 {
    fn mul_assign(&mut self, other: Vec2) {
        self.x *= other.x;
        self.y *= other.y;
    }
}

impl DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, other: f32) {
        self.x /= other;
        self.y /= other;
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, other: Vec2) {
        self.x -= other.x;
        self.y -= other.y;
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Div for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: f32) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Mul for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Vec2) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Vec2) -> bool {
        return self.x == other.x && self.y == other.y;
    }
}
impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let decimal_places = 3;

        let mut p = 0;
        let mut dp = 1.;
        while p < decimal_places {
            dp *= 10.;
            p += 1;
        }

        write!(
            f,
            "{}, {}",
            (self.x * dp).trunc() / dp,
            (self.y * dp).trunc() / dp
        )
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: Vec2,
    pub uv: Vec2,
}

#[derive(Clone, Default)]
pub struct Camera {
    pub pos: Vec2,
    pub zoom: f32,
    pub initial_pos: Vec2,
}

/// Input-related fields.
#[derive(Clone, Default)]
pub struct InputStates {
    pub modifier: i32,

    // mouse stuff
    pub initial_points: Vec<Vec2>,
    pub mouse_left: i32,
    pub mouse: Vec2,

    pub mouse_prev: Vec2,

    // is mouse on UI?
    pub on_ui: bool,

    pub pressed: Vec<KeyCode>,
}

#[derive(Clone, Default)]
pub struct Ui {
    pub edit_bar_pos: Vec2,
    pub animate_mode_bar_pos: Vec2,
    pub animate_mode_bar_scale: Vec2,

    pub rename_id: String,
    pub original_name: String,

    pub anim: UiAnim,
}

#[derive(Clone, Default)]
pub struct UiAnim {
    pub selected: usize,
    pub hovering_frame: i32,
    pub selected_frame: i32,
    pub timeline_zoom: f32,
    pub lines_x: Vec<f32>,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct Bone {
    pub id: i32,
    pub name: String,
    pub parent_id: i32,
    pub tex_idx: usize,

    /// used to properly offset bone's movement to counteract it's parent
    pub parent_rot: f32,

    pub rot: f32,
    pub scale: Vec2,
    pub pos: Vec2,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct Armature {
    pub bones: Vec<Bone>,
    pub animations: Vec<Animation>,

    #[serde(skip)]
    pub textures: Vec<Texture>,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct Texture {
    pub size: Vec2,
    pub pixels: Vec<u8>,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct Animation {
    pub name: String,
    pub fps: i32,
    pub keyframes: Vec<Keyframe>,
}

#[derive(serde::Serialize, Clone, Default)]
pub struct Keyframe {
    pub frame: i32,
    pub bones: Vec<AnimBone>,
}

#[derive(PartialEq, serde::Serialize, Clone, Default)]
pub struct AnimBone {
    pub id: i32,
    pub rot: f32,
    pub pos: Vec2,
    pub scale: Vec2,

    #[serde(skip)]
    pub pos_top: f32,
    #[serde(skip)]
    pub rot_top: f32,
}
pub struct BoneTops {
    pub id: i32,
    pub pos_top: f32,
    pub rot_top: f32,
}
#[derive(Default)]
pub struct Shared {
    pub window: Vec2,
    pub dragging: bool,
    pub selected_bone_idx: usize,
    pub armature: Armature,
    pub bind_groups: Vec<BindGroup>,
    pub camera: Camera,
    pub input: InputStates,
    pub egui_ctx: Context,
    pub cursor_icon: CursorIcon,
    pub ui: Ui,

    // tracking zoom every frame for smooth effect
    pub current_zoom: f32,

    // actual zoom
    pub zoom: f32,

    // should be enum but too lazy atm
    pub edit_mode: i32,

    pub animating: bool,

    /// useful if you don't want to provide an actual bind group during testing
    pub highlight_bindgroup: Option<BindGroup>,

    /// triggers debug stuff
    pub debug: bool,
}

// mostly for shorthands for cleaner code
impl Shared {
    pub fn selected_animation(&mut self) -> &mut Animation {
        &mut self.armature.animations[self.ui.anim.selected]
    }

    pub fn selected_keyframe(&mut self) -> Option<&mut Keyframe> {
        let frame = self.ui.anim.selected_frame;
        for kf in &mut self.selected_animation().keyframes {
            if kf.frame == frame {
                return Some(kf);
            }
        }
        None
    }

    pub fn selected_anim_bone(&mut self) -> Option<&mut AnimBone> {
        let id = self.armature.bones[self.selected_bone_idx].id;
        for b in &mut self.selected_keyframe().unwrap().bones {
            if b.id == id {
                return Some(b);
            }
        }
        None
    }

    pub fn selected_bone(&mut self) -> &mut Bone {
        &mut self.armature.bones[self.selected_bone_idx]
    }

    pub fn find_bone(&mut self, id: i32) -> Option<&Bone> {
        for b in &self.armature.bones {
            if b.id == id {
                return Some(&b);
            }
        }
        None
    }

    pub fn animate(&mut self, anim_idx: usize, frame: i32) -> Vec<Bone> {
        let mut bones = self.armature.bones.clone();
        for kf in &self.armature.animations[anim_idx].keyframes {
            // this frame exists
            if kf.frame == frame {
                for kf_b in &kf.bones {
                    for b in &mut bones {
                        if b.id == kf_b.id {
                            b.pos += kf_b.pos;
                        }
                    }
                }
            }
        }

        bones
    }

    pub fn move_with_mouse(&mut self, value: &Vec2, counter_parent: bool) -> Vec2 {
        // get mouse in world space
        let mut mouse_world = crate::utils::screen_to_world_space(self.input.mouse, self.window);
        mouse_world.x *= self.window.x / self.window.y;

        // Counter-act parent's rotation so that translation is global.
        // Only used in bone translation.
        if counter_parent {
            let parent_id = self.selected_bone().parent_id;
            if let Some(parent) = self.find_bone(parent_id) {
                mouse_world = crate::utils::rotate(&mouse_world, -parent.rot);
            }
        }

        // get initial values to allow 'dragging'
        if self.input.initial_points.len() == 0 {
            let initial = mouse_world * self.zoom;
            self.input.initial_points.push(*value - initial);
        }

        (mouse_world * self.zoom) + self.input.initial_points[0]
    }
}

impl Ui {
    pub fn check_renaming(
        &mut self,
        rename_id: &String,
        str: &mut String,
        ui: &mut egui::Ui,
    ) -> bool {
        if self.rename_id != *rename_id {
            return false;
        }
        let mut just_made = false;
        if self.original_name == "" {
            just_made = true;
            self.original_name = str.clone();
        }
        let text_edit = egui::TextEdit::singleline(str);
        let input = ui.add(text_edit.cursor_at_end(true));
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            *str = self.original_name.clone();
            self.rename_id = "".to_owned();
            self.original_name = "".to_owned();
        } else if input.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.rename_id = "".to_owned();
            self.original_name = "".to_owned();
        }
        if just_made {
            input.request_focus();
        }
        true
    }
}
