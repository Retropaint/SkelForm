//! Easily-accessible and frequently-shared data.

use std::{
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use egui::{text::CCursor, Context};
use tween::{Tween, Tweener};
use wgpu::BindGroup;
use winit::{keyboard::KeyCode, window::CursorIcon};

#[repr(C)]
#[derive(Debug, serde::Serialize, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl tween::TweenValue for Vec2 {
    fn scale(self, scale: f32) -> Self {
        self * scale
    }
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
    pub playing: bool,
    pub elapsed: i32,
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

#[derive(PartialEq, serde::Serialize, Clone, Default)]
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
        let id = self.selected_bone().id;
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

    pub fn animate(&mut self, _anim_idx: usize, frame: i32) -> Vec<Bone> {
        let mut bones = self.armature.bones.clone();

        // ignore if this animation has no keyframes
        let kf_len = self.selected_animation().keyframes.len();
        if kf_len == 0 {
            return bones;
        }

        // get the 2 frames between the chosen one to inerpolate
        let mut prev_kf_idx = usize::MAX;
        let mut next_kf_idx = usize::MAX;
        for (i, kf) in self.selected_animation().keyframes.iter().enumerate() {
            if kf.frame == frame {
                prev_kf_idx = i;
                next_kf_idx = i;
                break
            }
            if kf.frame <= frame {
                prev_kf_idx = i;
            } else if kf.frame > frame && next_kf_idx == usize::MAX {
                next_kf_idx = i;
            }
        }
        if prev_kf_idx == usize::MAX {
            prev_kf_idx = next_kf_idx;
        }
        if next_kf_idx == usize::MAX {
            next_kf_idx = prev_kf_idx;
        }

        let prev_kf = self.selected_animation().keyframes[prev_kf_idx].clone();
        let mut next_kf = self.selected_animation().keyframes[next_kf_idx].clone();

        // get the latest state of all bones that are prior to the ones being interpolated
        for i in (0..self.selected_animation().keyframes.len()).rev() {
            let kf = &self.selected_animation().keyframes[i];
            if kf.frame > prev_kf.frame {
                continue;
            }
            let mut idx: Vec<usize> = vec![];
            for (i, ab) in kf.bones.iter().enumerate() {
                idx.push(i);
                for b in &next_kf.bones {
                    if b.id == ab.id {
                        idx.pop();
                        break;
                    }
                }
            }
            for i in idx {
                next_kf.bones.push(kf.bones[i].clone());
            }
        }

        let mut tween_frames = next_kf.frame - prev_kf.frame;
        // Set total frames to 1 if there are none, 
        // as Tweener can't accept a duration of 0.
        if tween_frames == 0 {
            tween_frames = 1;
        }

        // get the current frame being pointed to
        let mut tween_current_frame = frame - prev_kf.frame;
        if tween_current_frame < 0 {
            tween_current_frame = 0;
        }

        for b in &mut bones {
            let mut prev_bone: Option<&AnimBone> = None;
            let mut next_bone: Option<&AnimBone> = None;
            for ab in &prev_kf.bones {
                if ab.id == b.id {
                    prev_bone = Some(&ab);
                }
            }
            for ab in &next_kf.bones {
                if ab.id == b.id {
                    next_bone = Some(&ab);
                }
            }

            if next_bone == None {
                next_bone = prev_bone;
            } else if prev_bone == None {
                prev_bone = next_bone;
            }

            if prev_bone != None || next_bone != None {
                // animate pos
                b.pos +=
                    Tweener::linear(prev_bone.unwrap().pos, next_bone.unwrap().pos, tween_frames)
                        .move_to(tween_current_frame);
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

        // Upon immediately clicking, track initial values to allow 'dragging'
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

        let default_name = "New Animation";

        // initialize input if it was just made
        let mut just_made = false;
        if self.original_name == "" {
            just_made = true;
            self.original_name = str.clone();
        }

        let input = ui.add(egui::TextEdit::singleline(str).hint_text(default_name));

        // immediately focus on this input if it was just made
        if just_made {
            input.request_focus();
        }

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            if str == "" && self.original_name == "" {
                *str = default_name.to_string();
            } else {
                *str = self.original_name.clone();
            }
            self.rename_id = "".to_owned();
            self.original_name = "".to_owned();
        } else if input.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if str == "" && self.original_name == "" {
                *str = default_name.to_string();
            } else if str == "" {
                *str = self.original_name.clone();
            }
            self.rename_id = "".to_owned();
            self.original_name = "".to_owned();
        }

        true
    }
}
