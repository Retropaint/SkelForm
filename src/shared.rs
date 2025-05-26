//! Easily-accessible and frequently-shared data.

use crate::*;

use std::{
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

pub const RECT_VERT_INDICES: [u32; 6] = [0, 1, 2, 1, 2, 3];
pub const NEW_BONE_NAME: &str = "New Bone";
pub const CLICK_THRESHOLD: i32 = 5;

use tween::Tweener;
use wgpu::BindGroup;
use winit::keyboard::KeyCode;

#[repr(C)]
#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Copy,
    Clone,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
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

impl Into<egui::Vec2> for Vec2 {
    fn into(self) -> egui::Vec2 {
        egui::Vec2::new(self.x, self.y)
    }
}

impl Vec2 {
    pub const ZERO: Self = Self::new(0., 0.);

    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
}

impl MulAssign for Vec2 {
    fn mul_assign(&mut self, other: Vec2) {
        self.x *= other.x;
        self.y *= other.y;
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, other: f32) {
        self.x *= other;
        self.y *= other;
    }
}

impl DivAssign for Vec2 {
    fn div_assign(&mut self, other: Vec2) {
        self.x /= other.x;
        self.y /= other.y;
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

impl Sub<f32> for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: f32) -> Self {
        Self {
            x: self.x - rhs,
            y: self.y - rhs,
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

/// enable enum names to be cast to string
macro_rules! enum_string {
    ($type:ty) => {
        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        }
    };
}

#[repr(C)]
#[derive(
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    Copy,
    Clone,
    bytemuck::Pod,
    bytemuck::Zeroable,
    Default,
)]
pub struct Vertex {
    pub pos: Vec2,
    pub uv: Vec2,
    #[serde(skip)]
    pub color: Color,
}

#[repr(C)]
#[derive(
    PartialEq, serde::Serialize, serde::Deserialize, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    pub const GREEN: Color = Color::new(0., 1., 0., 1.);
    pub const WHITE: Color = Color::new(1., 1., 1., 1.);
}

#[rustfmt::skip] 
impl Default for Color {
    fn default() -> Self {
        Color {  r: 1., g: 1., b: 1., a: 1. }
    }
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
    // mouse stuff
    pub mouse_left: i32,
    pub mouse_left_prev: i32,
    pub mouse_right: i32,
    pub mouse_right_prev: i32,
    pub mouse: Vec2,
    pub mouse_prev: Vec2,

    // is mouse on UI?
    pub on_ui: bool,

    pressed: Vec<KeyCode>,
}

impl InputStates {
    pub fn add_key(&mut self, key: &KeyCode) {
        let mut add = true;
        for pressed_key in &mut self.pressed {
            if key == pressed_key {
                add = false;
                break;
            }
        }
        if add {
            self.pressed.push(*key);
        }
    }

    pub fn remove_key(&mut self, key: &KeyCode) {
        for i in 0..self.pressed.len() {
            if *key == self.pressed[i] {
                self.pressed.remove(i);
                break;
            }
        }
    }
    /// Check if this key is being held down.
    pub fn is_pressing(&self, key: KeyCode) -> bool {
        for k in &self.pressed {
            if *k == key {
                return true;
            }
        }

        false
    }

    /// Check if this key was pressed.
    pub fn pressed(&mut self, key: KeyCode) -> bool {
        for i in 0..self.pressed.len() {
            if self.pressed[i] == key {
                self.pressed.remove(i);
                return true;
            }
        }

        false
    }

    pub fn clicked(&self) -> bool {
        self.mouse_left == -1
            && self.mouse_left_prev != -1
            && self.mouse_left_prev < CLICK_THRESHOLD
    }

    pub fn right_clicked(&self) -> bool {
        self.mouse_right == -1
            && self.mouse_right_prev != -1
            && self.mouse_right_prev < CLICK_THRESHOLD
    }

    pub fn is_clicking(&self) -> bool {
        self.mouse_left > 0
    }

    pub fn is_holding_click(&self) -> bool {
        self.mouse_left > CLICK_THRESHOLD
    }
}

#[derive(Clone, Default, PartialEq)]
pub enum UiState {
    #[default]
    ImageModal,
    Exiting,
    DraggingBone,
    RemovingTexture,
    ForcedModal,
    Modal,
    PolarModal,
    FirstTimeModal,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub enum PolarId {
    #[default]
    DeleteBone,
    Exiting,
    FirstTime,
}
enum_string!(PolarId);

#[derive(Clone, Default)]
pub struct Ui {
    pub anim: UiAnim,

    pub edit_bar_pos: Vec2,
    pub animate_mode_bar_pos: Vec2,
    pub animate_mode_bar_scale: Vec2,

    pub rename_id: String,
    pub original_name: String,

    // id to identify actions for polar (yes-no) dialog
    pub polar_id: PolarId,

    pub headline: String,

    // the initial value of what is being edited via input
    pub edit_value: Option<String>,

    pub texture_images: Vec<egui::TextureHandle>,

    pub states: Vec<UiState>,

    pub default_font_size: f32,

    pub scale: f32,

    // camera bar stuff
    pub camera_bar_pos: Vec2,
    pub camera_bar_scale: Vec2,
}

impl Ui {
    pub fn get_cursor(&self, ui: &egui::Ui) -> Vec2 {
        let cursor_pos = ui.ctx().input(|i| {
            if let Some(result) = i.pointer.hover_pos() {
                result
            } else {
                egui::Pos2::new(0., 0.)
            }
        });
        (cursor_pos - ui.min_rect().left_top()).into()
    }

    pub fn set_state(&mut self, state: UiState, add: bool) {
        if add {
            let mut already_added = false;
            for s in 0..self.states.len() {
                if self.states[s] == state {
                    already_added = true;
                    break;
                }
            }
            if !already_added {
                self.states.push(state);
            }
        } else {
            for s in 0..self.states.len() {
                if self.states[s] == state {
                    self.states.remove(s);
                    break;
                }
            }
        }
    }

    pub fn has_state(&self, state: UiState) -> bool {
        for s in 0..self.states.len() {
            if self.states[s] == state {
                return true;
            }
        }
        false
    }

    pub fn open_modal(&mut self, headline: String, forced: bool) {
        self.set_state(UiState::Modal, true);
        self.set_state(UiState::ForcedModal, forced);
        self.headline = headline;
    }

    pub fn open_polar_modal(&mut self, id: PolarId, headline: String) {
        self.set_state(UiState::PolarModal, true);
        self.polar_id = id;
        self.headline = headline;
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(skip)]
    pub first_launch: bool,

    #[serde(default = "default_one")]
    pub ui_scale: f32,
}

#[derive(Clone, Default)]
pub struct UiAnim {
    pub open: bool,
    pub selected: usize,
    pub hovering_frame: i32,
    pub selected_frame: i32,
    pub timeline_zoom: f32,
    pub lines_x: Vec<f32>,
    pub playing: bool,
    pub started: Option<chrono::DateTime<chrono::Utc>>,

    // the frame at which playing started
    pub played_frame: i32,

    pub exported_frame: String,

    pub timeline_offset: Vec2,
    pub dragged_keyframe: usize,
    pub icon_images: Vec<egui::TextureHandle>,
    pub loops: i32,

    pub bottom_bar_top: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default)]
pub struct Bone {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_neg_one")]
    pub parent_id: i32,
    #[serde(default = "default_neg_one")]
    pub tex_idx: i32,

    #[serde(default)]
    pub vertices: Vec<Vertex>,

    #[serde(default)]
    pub indices: Vec<u32>,

    /// used to properly offset bone's movement to counteract it's parent
    #[serde(skip)]
    pub parent_rot: f32,

    #[serde(default)]
    pub rot: f32,
    #[serde(default)]
    pub scale: Vec2,
    #[serde(default)]
    pub pos: Vec2,
    #[serde(default)]
    pub pivot: Vec2,
    #[serde(default)]
    pub zindex: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Armature {
    #[serde(default)]
    pub bones: Vec<Bone>,
    #[serde(default)]
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub textures: Vec<Texture>,
}

// used for the json
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Root {
    pub texture_size: Vec2,
    pub armatures: Vec<Armature>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Texture {
    #[serde(default)]
    pub offset: Vec2,
    #[serde(default)]
    pub size: Vec2,
    #[serde(default)]
    pub name: String,
    #[serde(skip)]
    pub pixels: Vec<u8>,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Animation {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub fps: i32,
    #[serde(default)]
    pub keyframes: Vec<Keyframe>,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Keyframe {
    #[serde(default)]
    pub frame: i32,
    //#[serde(default)]
    //pub bones: Vec<AnimBone>,
    #[serde(default)]
    pub bone_id: i32,
    #[serde(default)]
    pub element: AnimElement,

    // Only used in runtimes. Represents the element's index in the enum.
    #[serde(default)]
    pub element_id: i32,

    #[serde(default)]
    pub value: f32,

    #[serde(default)]
    pub transition: Transition,

    #[serde(skip)]
    pub label_top: f32,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default, Debug)]
pub enum Transition {
    #[default]
    Linear,
    SineIn,
    SineOut,
}

enum_string!(Transition);

#[derive(
    Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, Clone, Default, Debug,
)]
pub enum AnimElement {
    #[default]
    PositionX,
    PositionY,
    Rotation,
    ScaleX,
    ScaleY,
    PivotX,
    PivotY,
    Zindex,
}

pub const ANIM_ICON_ID: [usize; 7] = [0, 0, 1, 2, 2, 3, 3];

#[derive(Default, Clone, PartialEq)]
pub enum ActionEnum {
    #[default]
    Bone,
    Animation,
    Keyframe,
}
#[derive(Default, PartialEq, Clone)]
pub enum ActionType {
    #[default]
    Created,
    Edited,
}

#[derive(Default, Clone, PartialEq)]
pub struct Action {
    pub action: ActionEnum,
    pub action_type: ActionType,

    pub id: i32,
    pub animation: Animation,
    pub bone: Bone,
}

impl AnimElement {
    pub fn default_of(element: &AnimElement) -> f32 {
        match *element {
            AnimElement::ScaleX => {
                return 1.;
            }
            _ => 0.,
        }
    }
}

enum_string!(AnimElement);

#[derive(Default, Debug)]
pub struct BoneTops {
    pub tops: Vec<BoneTop>,
}

impl BoneTops {
    pub fn find(&self, id: i32, element: &AnimElement) -> Option<&BoneTop> {
        for bt in &self.tops {
            if bt.id == id && bt.element == *element {
                return Some(bt);
            }
        }
        None
    }

    pub fn find_mut(&mut self, id: i32, element: &AnimElement) -> Option<&mut BoneTop> {
        for bt in &mut self.tops {
            if bt.id == id && bt.element == *element {
                return Some(bt);
            }
        }
        None
    }

    pub fn find_bone(&self, id: i32) -> bool {
        for bt in &self.tops {
            if bt.id == id {
                return true;
            }
        }
        false
    }
}

#[derive(Default, PartialEq)]
pub enum EditMode {
    #[default]
    Move,
    Rotate,
    Scale,
}

#[derive(Default, PartialEq, Debug)]
pub struct BoneTop {
    pub id: i32,
    pub element: AnimElement,
    pub height: f32,
}
#[derive(Clone)]
pub struct RenderedFrame {
    pub buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
}

#[derive(Default, PartialEq, Clone)]
pub enum TutorialStep {
    NewBone,
    GetImage,
    EditBoneX,
    EditBoneY,
    OpenAnim,
    CreateAnim,
    SelectKeyframe,
    EditBone,

    // tutorial is meant to work with first bone only,
    // so it must be reselected to proceed
    ReselectBone,

    #[default]
    None,
}

#[derive(Default)]
pub struct Shared {
    pub window: Vec2,
    pub selected_bone_idx: usize,
    pub armature: Armature,
    pub bind_groups: Vec<BindGroup>,
    pub camera: Camera,
    pub input: InputStates,
    pub egui_ctx: egui::Context,
    pub cursor_icon: egui::CursorIcon,
    pub ui: Ui,
    pub editing_bone: bool,

    pub dragging_vert: usize,
    pub editing_mesh: bool,

    tutorial_step: TutorialStep,

    pub frame: i32,
    pub recording: bool,
    pub done_recording: bool,
    // mainly used for video, but can also be used for screenshots
    pub rendered_frames: Vec<RenderedFrame>,

    pub undo_actions: Vec<Action>,
    pub redo_actions: Vec<Action>,

    // should be enum but too lazy atm
    pub edit_mode: EditMode,

    pub generic_bindgroup: Option<BindGroup>,

    pub save_path: String,

    pub config: Config,

    /// triggers debug stuff. Set in main.rs
    pub debug: bool,
}

// mostly for shorthands for cleaner code
impl Shared {
    pub fn selected_animation(&self) -> Option<&Animation> {
        if self.ui.anim.selected > self.armature.animations.len() {
            return None;
        }
        Some(&self.armature.animations[self.ui.anim.selected])
    }

    pub fn selected_animation_mut(&mut self) -> Option<&mut Animation> {
        if self.ui.anim.selected > self.armature.animations.len() {
            return None;
        }
        Some(&mut self.armature.animations[self.ui.anim.selected])
    }

    pub fn selected_keyframe(&self) -> Option<&Keyframe> {
        if self.selected_animation() == None {
            return None;
        }
        let frame = self.ui.anim.selected_frame;
        for kf in &self.selected_animation().unwrap().keyframes {
            if kf.frame == frame {
                return Some(kf);
            }
        }
        None
    }

    pub fn selected_keyframe_mut(&mut self) -> Option<&mut Keyframe> {
        let frame = self.ui.anim.selected_frame;
        for kf in &mut self.selected_animation_mut().unwrap().keyframes {
            if kf.frame == frame {
                return Some(kf);
            }
        }
        None
    }

    pub fn unselect_everything(&mut self) {
        self.selected_bone_idx = usize::MAX;
        self.ui.anim.selected_frame = 0;
        self.editing_mesh = false;
        self.ui.anim.selected = usize::MAX;
    }

    pub fn select_bone(&mut self, idx: usize) {
        let selected_anim = self.ui.anim.selected;
        self.unselect_everything();
        self.ui.anim.selected = selected_anim;
        self.selected_bone_idx = idx;

        if self.tutorial_step == TutorialStep::None {
            return;
        }

        // guide user to select first bone again in tutorial
        if idx != 0 {
            self.tutorial_step = TutorialStep::ReselectBone;
        } else {
            self.tutorial_step = self.next_tutorial_step(TutorialStep::NewBone);
        }
    }

    pub fn select_frame(&mut self, idx: i32) {
        let selected_anim = self.ui.anim.selected;
        self.unselect_everything();
        self.ui.anim.selected = selected_anim;
        self.ui.anim.selected_frame = idx;
    }

    pub fn sort_keyframes(&mut self) {
        self.selected_animation_mut()
            .unwrap()
            .keyframes
            .sort_by(|a, b| a.frame.cmp(&b.frame));
    }

    pub fn last_keyframe(&self) -> Option<&Keyframe> {
        self.selected_animation().unwrap().keyframes.last()
    }

    pub fn last_keyframe_mut(&mut self) -> Option<&mut Keyframe> {
        self.selected_animation_mut().unwrap().keyframes.last_mut()
    }

    pub fn keyframe(&self, idx: usize) -> Option<&Keyframe> {
        if idx > self.selected_animation().unwrap().keyframes.len() - 1 {
            return None;
        }
        Some(&self.selected_animation().unwrap().keyframes[idx])
    }

    pub fn keyframe_mut(&mut self, idx: usize) -> Option<&mut Keyframe> {
        if idx > self.selected_animation().unwrap().keyframes.len() - 1 {
            return None;
        }
        Some(&mut self.selected_animation_mut().unwrap().keyframes[idx])
    }

    pub fn keyframe_at(&self, frame: i32) -> Option<&Keyframe> {
        for kf in &self.selected_animation().unwrap().keyframes {
            if kf.frame == frame {
                return Some(&kf);
            }
        }

        None
    }

    pub fn keyframe_at_mut(&mut self, frame: i32) -> Option<&mut Keyframe> {
        for kf in &mut self.selected_animation_mut().unwrap().keyframes {
            if kf.frame == frame {
                return Some(kf);
            }
        }

        None
    }

    pub fn selected_bone(&self) -> Option<&Bone> {
        if self.selected_bone_idx != usize::MAX {
            return Some(&self.armature.bones[self.selected_bone_idx]);
        }
        None
    }

    pub fn selected_bone_mut(&mut self) -> Option<&mut Bone> {
        if self.selected_bone_idx != usize::MAX {
            return Some(&mut self.armature.bones[self.selected_bone_idx]);
        }
        None
    }

    pub fn find_bone(&self, id: i32) -> Option<&Bone> {
        for b in &self.armature.bones {
            if b.id == id {
                return Some(&b);
            }
        }
        None
    }

    pub fn delete_bone(&mut self, id: i32) {
        for i in 0..self.armature.bones.len() {
            let bone_id = self.armature.bones[i].id;
            if bone_id == id {
                self.armature.bones.remove(i);
                break;
            }
        }
    }

    pub fn find_bone_mut(&mut self, id: i32) -> Option<&mut Bone> {
        for b in &mut self.armature.bones {
            if b.id == id {
                return Some(b);
            }
        }
        None
    }

    pub fn animate(&self, _anim_idx: usize) -> Vec<Bone> {
        let mut bones = self.armature.bones.clone();

        // ignore if this animation has no keyframes
        let kf_len = self.selected_animation().unwrap().keyframes.len();
        if kf_len == 0 {
            return bones;
        }

        for b in &mut bones {
            macro_rules! interpolate {
                ($element:expr, $default:expr) => {{
                    let (prev, next, total_frames, current_frame, transition) = self
                        .find_connecting_frames(
                            b.id,
                            $element,
                            $default,
                            self.ui.anim.selected_frame,
                        );
                    match (transition) {
                        Transition::SineIn => {
                            Tweener::sine_in(prev, next, total_frames).move_to(current_frame)
                        }
                        Transition::SineOut => {
                            Tweener::sine_out(prev, next, total_frames).move_to(current_frame)
                        }
                        _ => Tweener::linear(prev, next, total_frames).move_to(current_frame),
                    }
                }};
            }

            // interpolate!
            #[rustfmt::skip]
            {
                b.pos.x   += interpolate!(AnimElement::PositionX, 0.);
                b.pos.y   += interpolate!(AnimElement::PositionY, 0.);
                b.rot     += interpolate!(AnimElement::Rotation,  0.);
                b.scale.x *= interpolate!(AnimElement::ScaleX,    1.);
                b.scale.y *= interpolate!(AnimElement::ScaleY,    1.);
                b.pivot.x += interpolate!(AnimElement::PivotX,    0.);
                b.pivot.y += interpolate!(AnimElement::PivotY,    0.);
                b.zindex  += interpolate!(AnimElement::Zindex,    0.);
            };
        }

        bones
    }

    pub fn find_connecting_frames(
        &self,
        bone_id: i32,
        element: AnimElement,
        default: f32,
        frame: i32,
    ) -> (f32, f32, i32, i32, Transition) {
        let mut prev: Option<f32> = None;
        let mut next: Option<f32> = None;
        let mut start_frame = 0;
        let mut end_frame = 0;
        let mut transition: Transition = Transition::Linear;

        // get most previous frame with this element
        for (i, kf) in self
            .selected_animation()
            .unwrap()
            .keyframes
            .iter()
            .enumerate()
        {
            if self.selected_animation().unwrap().keyframes[i].frame > frame {
                break;
            }

            if kf.bone_id != bone_id || kf.element != element {
                continue;
            }

            prev = Some(kf.value);
            start_frame = kf.frame;
        }

        // get first next frame with this element
        for (i, kf) in self
            .selected_animation()
            .unwrap()
            .keyframes
            .iter()
            .enumerate()
            .rev()
        {
            if self.selected_animation().unwrap().keyframes[i].frame < frame {
                break;
            }

            if kf.bone_id != bone_id || kf.element != element {
                continue;
            }

            next = Some(kf.value);
            end_frame = kf.frame;
            transition = kf.transition.clone();
        }

        // ensure prev and next are pointing somewhere
        if prev == None {
            if next != None {
                prev = next
            } else {
                prev = Some(default)
            }
        }
        if next == None {
            if prev != None {
                next = prev;
            } else {
                next = Some(default);
            }
        }

        let mut total_frames = end_frame - start_frame;
        // Tweener doesn't accept 0 duration
        if total_frames == 0 {
            total_frames = 1;
        }

        let current_frame = frame - start_frame;

        (
            prev.unwrap(),
            next.unwrap(),
            total_frames,
            current_frame,
            transition,
        )
    }

    pub fn save_edited_bone(&mut self) {
        self.undo_actions.push(Action {
            action: ActionEnum::Bone,
            action_type: ActionType::Edited,
            bone: self.selected_bone().unwrap().clone(),
            id: self.selected_bone().unwrap().id,
            ..Default::default()
        });

        if self.is_animating() {
            self.undo_actions.push(Action {
                action: ActionEnum::Animation,
                action_type: ActionType::Edited,
                id: self.ui.anim.selected as i32,
                animation: self.selected_animation().unwrap().clone(),
                ..Default::default()
            });
        }
    }

    pub fn edit_bone(&mut self, element: &AnimElement, mut value: f32, overwrite: bool) {
        let is_animating = self.is_animating();

        macro_rules! edit {
            ($field:expr) => {
                if !is_animating {
                    $field = value;
                } else if overwrite {
                    // if overwriting, modify the value such that it will return to the current field's value on animating
                    match(element) {
                        AnimElement::ScaleX | AnimElement::ScaleY=> value /= $field,
                        _ => value -= $field
                    }
                }
            };
        }

        let bone_mut = self.selected_bone_mut().unwrap();

        #[rustfmt::skip]
        match element {
            AnimElement::PositionX => { edit!(bone_mut.pos.x);   },
            AnimElement::PositionY => { edit!(bone_mut.pos.y);   },
            AnimElement::Rotation =>  { edit!(bone_mut.rot);     },
            AnimElement::ScaleX =>    { edit!(bone_mut.scale.x); },
            AnimElement::ScaleY =>    { edit!(bone_mut.scale.y); },
            AnimElement::PivotX =>    { edit!(bone_mut.pivot.x); },
            AnimElement::PivotY =>    { edit!(bone_mut.pivot.y); },
            AnimElement::Zindex =>    { edit!(bone_mut.zindex);  },
        };

        if !self.is_animating() {
            return;
        }

        // create keyframe at 0th frame for this element if it doesn't exist
        if self.ui.anim.selected_frame != 0 {
            self.check_if_in_keyframe(self.selected_bone().unwrap().id, 0, element.clone());
        }

        let frame = self.check_if_in_keyframe(
            self.selected_bone().unwrap().id,
            self.ui.anim.selected_frame,
            element.clone(),
        );
        self.selected_animation_mut().unwrap().keyframes[frame].value = value;

        self.sort_keyframes();
    }

    /// Return which frame has these attributes, or create a new one
    fn check_if_in_keyframe(&mut self, id: i32, frame: i32, element: AnimElement) -> usize {
        // check if this keyframe exists
        let mut exists_at = usize::MAX;
        for i in 0..self.selected_animation().unwrap().keyframes.len() {
            let kf = &self.selected_animation().unwrap().keyframes[i];
            if kf.frame == frame && kf.bone_id == id && kf.element == element {
                exists_at = i;
                break;
            }
        }

        if exists_at != usize::MAX {
            return exists_at;
        }

        self.selected_animation_mut()
            .unwrap()
            .keyframes
            .push(Keyframe {
                frame,
                bone_id: id,
                element,
                ..Default::default()
            });

        self.selected_animation().unwrap().keyframes.len() - 1
    }

    pub fn is_animating(&self) -> bool {
        self.ui.anim.open && self.ui.anim.selected != usize::MAX
    }

    pub fn remove_texture(&mut self, tex_idx: i32) {
        self.armature.textures.remove(tex_idx as usize);
        self.bind_groups.remove(tex_idx as usize);
        let _ = self.ui.texture_images.remove(tex_idx as usize);
        for bone in &mut self.armature.bones {
            if bone.tex_idx == tex_idx {
                bone.tex_idx = -1;
            }
            if bone.tex_idx > tex_idx {
                bone.tex_idx -= 1;
            }
        }
    }

    pub fn sort_bone_zindex(&mut self, bone_idx: i32) {
        self.armature.bones[bone_idx as usize].zindex = bone_idx as f32 + 1.;
    }

    /// place child bone underneath its parent
    pub fn organize_bone(&mut self, bone_idx: usize) {
        let parent_id = self.armature.bones[bone_idx].parent_id;
        let bone = self.armature.bones[bone_idx].clone();
        let mut new_idx = bone_idx;
        for (i, bone) in self.armature.bones.iter().enumerate() {
            if parent_id == bone.id {
                new_idx = i;
                break;
            }
        }

        if new_idx != bone_idx {
            self.armature.bones.remove(bone_idx);
            self.armature.bones.insert(new_idx, bone);
        }
    }

    pub fn mouse_vel(&self) -> Vec2 {
        let mouse_world = utils::screen_to_world_space(self.input.mouse, self.window);
        let mouse_prev_world = utils::screen_to_world_space(self.input.mouse_prev, self.window);
        mouse_prev_world - mouse_world
    }

    pub fn start_tutorial(&mut self) {
        if self.selected_bone_idx != 0 && self.selected_bone_idx != usize::MAX {
            self.tutorial_step = TutorialStep::ReselectBone;
        } else {
            self.tutorial_step = TutorialStep::NewBone;
            self.tutorial_step = self.next_tutorial_step(self.tutorial_step.clone());
        }
    }

    pub fn start_next_tutorial_step(&mut self, next: TutorialStep) {
        if next as usize == self.tutorial_step.clone() as usize + 1 {
            self.tutorial_step = self.next_tutorial_step(self.tutorial_step.clone());
        }
    }

    /// Recursively check which tutorial step is next to show
    pub fn next_tutorial_step(&mut self, step: TutorialStep) -> TutorialStep {
        if self.tutorial_step == TutorialStep::None {
            return TutorialStep::None;
        }

        macro_rules! check {
            ($bool:expr, $next:expr) => {
                if $bool {
                    self.next_tutorial_step($next)
                } else {
                    step
                }
            };
        }

        let mut first_bone = &Bone::default();
        let bones_len = self.armature.bones.len();
        if bones_len > 0 {
            first_bone = &self.armature.bones[0];
        }
        let anim_selected = self.ui.anim.selected != usize::MAX;

        #[rustfmt::skip]
        let final_step = match step {
            TutorialStep::NewBone        => check!(bones_len > 0,            TutorialStep::GetImage),
            TutorialStep::GetImage       => check!(first_bone.tex_idx != -1, TutorialStep::EditBoneX),
            TutorialStep::EditBoneX      => check!(first_bone.pos.x != 0.,   TutorialStep::EditBoneY),
            TutorialStep::EditBoneY      => check!(first_bone.pos.y != 0.,   TutorialStep::OpenAnim),
            TutorialStep::OpenAnim       => check!(self.ui.anim.open,        TutorialStep::CreateAnim),
            TutorialStep::CreateAnim     => check!(anim_selected,            TutorialStep::SelectKeyframe),
            TutorialStep::SelectKeyframe => step,
            _ => step
        };
        final_step
    }

    pub fn set_tutorial_step(&mut self, step: TutorialStep) {
        if !self.tutorial_step_is(TutorialStep::None) {
            self.tutorial_step = step;
        }
    }

    pub fn tutorial_step_is(&self, step: TutorialStep) -> bool {
        self.tutorial_step == step
    }
}

impl Ui {
    pub fn check_renaming<T: FnOnce(bool)>(
        &mut self,
        rename_id: &String,
        str: &mut String,
        ui: &mut egui::Ui,
        after_enter: T,
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
            if self.original_name == "".to_string() {
                self.original_name = default_name.to_string();
            }
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
            after_enter(false);
        } else if input.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if str == "" && self.original_name == "" {
                *str = default_name.to_string();
            } else if str == "" {
                *str = self.original_name.clone();
            }
            self.rename_id = "".to_owned();
            self.original_name = "".to_owned();
            after_enter(true);
        }

        true
    }

    pub fn singleline_input(&mut self, id: String, mut value: f32, ui: &mut egui::Ui) -> f32 {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if self.edit_value != None && self.rename_id == id {
                let mut string = self.edit_value.clone().unwrap();
                let input = ui.add_sized([40., 20.], egui::TextEdit::singleline(&mut string));
                self.edit_value = Some(string);
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    input.surrender_focus();
                    if let Ok(f) = self.edit_value.clone().unwrap().parse::<f32>() {
                        value = f;
                    } else {
                        value = 0.;
                    }
                    self.edit_value = None;
                }
            } else {
                let mut string = value.to_string();
                let input = ui.add_sized([40., 20.], egui::TextEdit::singleline(&mut string));
                if input.gained_focus() {
                    self.rename_id = id;
                    self.edit_value = Some(value.to_string());
                }
            }
        });
        value
    }
}

fn default_neg_one() -> i32 {
    -1
}

fn default_one() -> f32 {
    1.
}
