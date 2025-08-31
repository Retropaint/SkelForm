//! Easily-accessible and frequently-shared data.

use crate::*;

use std::{
    fmt,
    ops::{DivAssign, MulAssign},
};

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

    pub fn mag(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Vec2 {
        let mag = self.mag();
        Vec2::new(self.x / mag, self.y / mag)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, other: f32) {
        self.x *= other;
        self.y *= other;
    }
}

impl DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, other: f32) {
        self.x /= other;
        self.y /= other;
    }
}

macro_rules! impl_assign_for_vec2 {
    ($trait:ident, $method:ident, $op:tt) => {
        impl std::ops::$trait for Vec2 {
            fn $method(&mut self, other: Vec2) {
                self.x $op other.x;
                self.y $op other.y;
            }
        }
    };
}

impl_assign_for_vec2!(AddAssign, add_assign, +=);
impl_assign_for_vec2!(SubAssign, sub_assign, -=);
impl_assign_for_vec2!(DivAssign, div_assign, /=);
impl_assign_for_vec2!(MulAssign, mul_assign, *=);

macro_rules! impl_for_vec2 {
    ($trait:ident, $method:ident, $op:tt) => {
        impl std::ops::$trait for Vec2 {
            type Output = Self;

            #[inline(always)]
            fn $method(self, rhs: Self) -> Self {
                Self {
                    x: self.x $op rhs.x,
                    y: self.y $op rhs.y,
                }
            }
        }
    };
}

impl_for_vec2!(Add, add, +);
impl_for_vec2!(Sub, sub, -);
impl_for_vec2!(Mul, mul, *);
impl_for_vec2!(Div, div, /);

macro_rules! impl_f32_for_vec2 {
    ($trait:ident, $method:ident, $op:tt) => {
        impl std::ops::$trait<f32> for Vec2 {
            type Output = Self;

            #[inline(always)]
            fn $method(self, rhs: f32) -> Self {
                Self {
                    x: self.x $op rhs,
                    y: self.y $op rhs,
                }
            }
        }
    };
}

impl_f32_for_vec2!(Sub, sub, -);
impl_f32_for_vec2!(Mul, mul, *);
impl_f32_for_vec2!(Div, div, /);

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
    Debug,
)]
pub struct Vertex {
    pub pos: Vec2,
    pub uv: Vec2,
    #[serde(skip)]
    pub color: VertexColor,
    #[serde(skip)]
    pub add_color: VertexColor,
}

impl Default for Vertex {
    fn default() -> Self {
        Vertex {
            pos: Vec2::default(),
            uv: Vec2::default(),
            color: VertexColor::default(),
            add_color: VertexColor::new(0., 0., 0., 0.),
        }
    }
}

#[repr(C)]
#[derive(PartialEq, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct VertexColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl VertexColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> VertexColor {
        VertexColor { r, g, b, a }
    }

    pub const GREEN: VertexColor = VertexColor::new(0., 1., 0., 1.);
    pub const WHITE: VertexColor = VertexColor::new(1., 1., 1., 1.);
}

#[rustfmt::skip]
impl Default for VertexColor {
    fn default() -> Self {
        VertexColor {  r: 1., g: 1., b: 1., a: 1. }
    }
}

#[repr(C)]
#[derive(
    PartialEq,
    Copy,
    Clone,
    bytemuck::Pod,
    bytemuck::Zeroable,
    Debug,
    serde::Deserialize,
    serde::Serialize,
)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(skip)]
    pub a: u8,
}

impl std::ops::AddAssign for Color {
    fn add_assign(&mut self, other: Color) {
        self.r += other.r;
        self.g += other.g;
        self.b += other.b;
        self.a += other.a;
    }
}

impl std::ops::SubAssign for Color {
    fn sub_assign(&mut self, other: Color) {
        self.r -= other.r;
        self.g -= other.g;
        self.b -= other.b;
        self.a -= other.a;
    }
}
impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }
}

impl From<egui::Color32> for Color {
    fn from(col: egui::Color32) -> Color {
        Color::new(col.r(), col.g(), col.b(), col.a())
    }
}

impl Into<egui::Color32> for Color {
    fn into(self) -> egui::Color32 {
        egui::Color32::from_rgb(self.r, self.g, self.b)
    }
}

#[rustfmt::skip]
impl Default for Color {
    fn default() -> Self {
        Color {  r: 0, g: 0, b: 0, a: 255 }
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
    pub last_pressed: Option<egui::Key>,
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
    SettingsModal,
    StartupWindow,
}

#[derive(Clone, Default, PartialEq)]
pub enum SettingsState {
    #[default]
    Ui,
    Rendering,
    Keyboard,
    Misc,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub enum PolarId {
    #[default]
    DeleteBone,
    Exiting,
    FirstTime,
    DeleteAnim,
    DeleteFile,
}
enum_string!(PolarId);

#[derive(Clone, Default, PartialEq)]
pub enum ContextType {
    #[default]
    None,
    Animation,
    Bone,
}

#[derive(Clone, Default)]
pub struct ContextMenu {
    pub context_type: ContextType,
    pub id: i32,
    pub hide: bool,
    pub keep: bool,
}

impl ContextMenu {
    pub fn show(&mut self, context_type: ContextType, id: i32) {
        self.context_type = context_type;
        self.id = id;
        self.hide = false;
    }

    pub fn close(&mut self) {
        self.context_type = ContextType::None;
        self.keep = false;
    }

    pub fn is(&self, context_type: ContextType, id: i32) -> bool {
        self.context_type == context_type && self.id == id && !self.hide
    }
}

#[derive(Clone, Default)]
pub struct Ui {
    pub anim: UiAnim,

    pub edit_bar_pos: Vec2,
    pub animate_mode_bar_pos: Vec2,
    pub animate_mode_bar_scale: Vec2,

    pub selected_bone_idx: usize,
    pub editing_mesh: bool,

    pub tutorial_step: TutorialStep,

    pub rename_id: String,
    pub original_name: String,

    // id to identify actions for polar (yes-no) dialog
    pub polar_id: PolarId,

    pub headline: String,

    // the initial value of what is being edited via input
    pub edit_value: Option<String>,

    pub states: Vec<UiState>,

    pub scale: f32,

    /// Ensures that auto-focused behaviour only runs once
    pub input_focused: bool,

    // camera bar stuff
    pub camera_bar_pos: Vec2,
    pub camera_bar_scale: Vec2,

    // context menu stuff

    // determines if context menu should close on next click
    pub context_menu: ContextMenu,

    pub settings_state: SettingsState,

    pub changing_key: String,

    pub selected_tex_set_idx: i32,

    pub hovering_tex: i32,

    pub showing_samples: bool,

    pub selected_path: String,
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

    pub fn open_polar_modal(&mut self, id: PolarId, headline: &str) {
        self.set_state(UiState::PolarModal, true);
        self.polar_id = id;
        self.headline = headline.to_string();
    }

    pub fn set_tutorial_step(&mut self, step: TutorialStep) {
        if !self.tutorial_step_is(TutorialStep::None) {
            self.tutorial_step = step;
        }
    }

    pub fn tutorial_step_is(&self, step: TutorialStep) -> bool {
        self.tutorial_step == step
    }

    pub fn start_tutorial(&mut self, armature: &Armature) {
        if self.selected_bone_idx != 0 && self.selected_bone_idx != usize::MAX {
            self.tutorial_step = TutorialStep::ReselectBone;
        } else {
            self.tutorial_step = TutorialStep::NewBone;
            self.tutorial_step = self.next_tutorial_step(self.tutorial_step.clone(), armature);
        }
    }

    pub fn start_next_tutorial_step(&mut self, next: TutorialStep, armature: &Armature) {
        if next as usize == self.tutorial_step.clone() as usize + 1 {
            self.tutorial_step = self.next_tutorial_step(self.tutorial_step.clone(), armature);
        }
    }

    pub fn next_tutorial_step(&mut self, step: TutorialStep, armature: &Armature) -> TutorialStep {
        if self.tutorial_step == TutorialStep::None {
            return TutorialStep::None;
        }

        macro_rules! check {
            ($bool:expr, $next:expr) => {
                if $bool {
                    self.next_tutorial_step($next, armature)
                } else {
                    step
                }
            };
        }

        let mut fb = &Bone::default();
        let bones_len = armature.bones.len();
        if bones_len > 0 {
            fb = &armature.bones[0];
        }
        let anim_selected = self.anim.selected != usize::MAX;
        let has_anim = armature.animations.len() > 0 && armature.animations[0].keyframes.len() > 0;
        let selected_frame = self.anim.selected_frame != 0;

        // iterable tutorial steps
        #[rustfmt::skip]
        let final_step = match step {
            TutorialStep::NewBone        => check!(bones_len > 0,      TutorialStep::GetImage),
            TutorialStep::GetImage       => check!(fb.tex_idx != -1,   TutorialStep::EditBoneX),
            TutorialStep::EditBoneX      => check!(fb.pos.x != 0.,     TutorialStep::EditBoneY),
            TutorialStep::EditBoneY      => check!(fb.pos.y != 0.,     TutorialStep::OpenAnim),
            TutorialStep::OpenAnim       => check!(self.anim.open,     TutorialStep::CreateAnim),
            TutorialStep::CreateAnim     => check!(anim_selected,      TutorialStep::SelectKeyframe),
            TutorialStep::SelectKeyframe => check!(selected_frame,     TutorialStep::EditBoneAnim),
            TutorialStep::EditBoneAnim   => check!(has_anim,           TutorialStep::PlayAnim),
            // TutorialStep::PlayAnim       => check!(self.anim.playing,  TutorialStep::StopAnim),
            // TutorialStep::StopAnim       => check!(!self.anim.playing, TutorialStep::Finish),
            _ => step
        };
        final_step
    }

    pub fn unselect_everything(&mut self) {
        self.selected_bone_idx = usize::MAX;
        self.anim.selected_frame = -1;
        self.editing_mesh = false;
        self.anim.selected = usize::MAX;
    }

    pub fn is_animating(&self) -> bool {
        self.anim.open && self.anim.selected != usize::MAX
    }

    pub fn select_anim_frame(&mut self, idx: i32) {
        let selected_anim = self.anim.selected;
        self.unselect_everything();
        self.anim.selected = selected_anim;
        self.anim.selected_frame = idx;
    }

    pub fn select_bone(&mut self, idx: usize, armature: &Armature) {
        let selected_anim = self.anim.selected;
        self.unselect_everything();
        self.anim.selected = selected_anim;
        self.selected_bone_idx = idx;

        if self.tutorial_step == TutorialStep::None {
            return;
        }

        // guide user to select first bone again in tutorial
        if idx != 0 {
            self.tutorial_step = TutorialStep::ReselectBone;
        } else {
            self.tutorial_step = self.next_tutorial_step(TutorialStep::NewBone, &armature);
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Config {
    #[serde(skip)]
    pub first_time: bool,

    #[serde(default = "default_one")]
    pub ui_scale: f32,

    #[serde(default = "gridline_default")]
    pub gridline_gap: i32,

    #[serde(default)]
    pub ui_colors: ColorConfig,
    #[serde(default)]
    pub keys: KeyboardConfig,

    #[serde(default)]
    pub hide_startup: bool,

    #[serde(default)]
    pub autosave_frequency: i32,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct ColorConfig {
    pub main: Color,
    pub light_accent: Color,
    pub dark_accent: Color,
    pub text: Color,
    pub frameline: Color,
    pub gradient: Color,
    pub background: Color,
    pub gridline: Color,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            first_time: true,
            ui_scale: default_one(),
            ui_colors: ColorConfig::default(),
            keys: KeyboardConfig::default(),
            gridline_gap: gridline_default(),
            hide_startup: false,
            autosave_frequency: 5,
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        ColorConfig {
            main: Color::new(32, 25, 46, 255),
            light_accent: Color::new(65, 46, 105, 255),
            dark_accent: Color::new(44, 36, 64, 255),
            text: Color::new(180, 180, 180, 255),
            frameline: Color::new(80, 60, 130, 255),
            gradient: Color::new(28, 20, 42, 255),
            background: Color::new(50, 50, 50, 255),
            gridline: Color::new(150, 150, 150, 255),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct KeyboardConfig {
    pub next_anim_frame: egui::KeyboardShortcut,
    pub prev_anim_frame: egui::KeyboardShortcut,
    pub zoom_in_camera: egui::KeyboardShortcut,
    pub zoom_out_camera: egui::KeyboardShortcut,
    pub undo: egui::KeyboardShortcut,
    pub redo: egui::KeyboardShortcut,
    pub save: egui::KeyboardShortcut,
    pub open: egui::KeyboardShortcut,
    pub cancel: egui::KeyboardShortcut,
}

pub trait Display {
    fn display(self) -> String;
}

impl Display for egui::KeyboardShortcut {
    /// Return this shortcut as a presentable string.
    fn display(self) -> String {
        let mut str: Vec<String> = self
            .format(&egui::ModifierNames::SYMBOLS, cfg!(target_os = "macos"))
            .chars()
            .map(|c| c.to_string())
            .collect();

        // replace mod sybols with names for now, since egui default font doesn't have them
        for key in &mut str {
            *key = key.replace("⌥", "Opt");
            *key = key.replace("⌃", "Ctrl");
            *key = key.replace("⇧", "Shift");
        }

        str.join(" ")
    }
}

impl Display for egui::Key {
    fn display(self) -> String {
        match self {
            egui::Key::F31 => "M1",
            egui::Key::F32 => "M2",
            egui::Key::F33 => "M3",
            egui::Key::F34 => "M4",
            egui::Key::F35 => "M5",
            _ => self.symbol_or_name(),
        }
        .to_string()
    }
}

macro_rules! regular_key {
    ($key:expr) => {
        egui::KeyboardShortcut::new(egui::Modifiers::NONE, $key)
    };
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        KeyboardConfig {
            next_anim_frame: regular_key!(egui::Key::ArrowRight),
            prev_anim_frame: regular_key!(egui::Key::ArrowLeft),
            zoom_in_camera: regular_key!(egui::Key::Equals),
            zoom_out_camera: regular_key!(egui::Key::Minus),
            undo: egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Z),
            redo: egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::Y),
            save: egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S),
            open: egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::O),
            cancel: regular_key!(egui::Key::Escape),
        }
    }
}

#[derive(Clone, Default)]
pub enum Keys {
    #[default]
    None,
    NextAnimFrame,
    PrevAnimFrame,
    ZoomInCamera,
    ZoomOutCamera,
    ZoomOutUi,
    ZoomInUi,
    Undo,
    Redo,
    Save,
    Open,
    Cancel,
}

#[derive(Clone, Default)]
pub struct UiAnim {
    pub open: bool,
    pub selected: usize,
    pub hovering_frame: i32,
    pub selected_frame: i32,
    pub timeline_zoom: f32,
    pub lines_x: Vec<f32>,

    // the frame at which playing started
    pub played_frame: i32,

    pub exported_frame: String,

    pub timeline_offset: Vec2,
    pub dragged_keyframe: Keyframe,
    pub icon_images: Vec<egui::TextureHandle>,
    pub loops: i32,

    pub bottom_bar_top: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq, Debug)]
pub enum JointEffector {
    #[default]
    None,
    Start,
    Middle,
    End,
}
enum_string!(JointEffector);

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq, Debug)]
pub enum JointConstraint {
    #[default]
    None,
    Clockwise,
    CounterClockwise,
}

enum_string!(JointConstraint);

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default, Debug)]
pub struct Bone {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_neg_one")]
    pub parent_id: i32,
    #[serde(default = "default_neg_one")]
    pub tex_set_idx: i32,
    #[serde(default = "default_neg_one")]
    pub tex_idx: i32,

    #[serde(default, skip_serializing_if = "are_verts_empty")]
    pub vertices: Vec<Vertex>,

    #[serde(default, skip_serializing_if = "are_indices_empty")]
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
    #[serde(default)]
    pub joint_effector: JointEffector,
    #[serde(default)]
    pub constraint: JointConstraint,

    #[serde(default)]
    pub hidden: bool,

    #[serde(skip)]
    pub folded: bool,
    #[serde(skip)]
    pub joint_folded: bool,
    #[serde(skip)]
    pub aiming: bool,
    #[serde(skip)]
    pub world_verts: Vec<Vertex>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Armature {
    #[serde(default)]
    pub bones: Vec<Bone>,
    #[serde(default, skip_serializing_if = "are_anims_empty")]
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub texture_sets: Vec<TextureSet>,

    #[serde(skip)]
    pub tex_sheet_buf: Vec<u8>,
}

impl Armature {
    pub fn find_bone(&self, id: i32) -> Option<&Bone> {
        for b in &self.bones {
            if b.id == id {
                return Some(&b);
            }
        }
        None
    }

    pub fn find_bone_idx(&self, id: i32) -> Option<usize> {
        for i in 0..self.bones.len() {
            if self.bones[i].id == id {
                return Some(i);
            }
        }
        None
    }

    pub fn find_bone_mut(&mut self, id: i32) -> Option<&mut Bone> {
        for b in &mut self.bones {
            if b.id == id {
                return Some(b);
            }
        }
        None
    }

    pub fn find_anim(&self, id: i32) -> Option<&Animation> {
        for a in &self.animations {
            if a.id == id {
                return Some(&a);
            }
        }
        None
    }

    pub fn find_anim_mut(&mut self, id: i32) -> Option<&mut Animation> {
        for a in &mut self.animations {
            if a.id == id {
                return Some(a);
            }
        }
        None
    }

    pub fn set_bone_tex(
        &mut self,
        bone_id: i32,
        new_tex_idx: usize,
        tex_set_idx: i32,
        selected_anim: usize,
        selected_frame: i32,
    ) {
        let tex_idx = self.find_bone(bone_id).unwrap().tex_idx;

        if selected_anim == usize::MAX {
            self.find_bone_mut(bone_id).unwrap().tex_idx = new_tex_idx as i32;
            self.find_bone_mut(bone_id).unwrap().tex_set_idx = tex_set_idx;

            // Set bone's verts to match texture.
            // Original logic checks if verts were edited, but this is temporarily disabled
            // for consistency with animations.
            if tex_idx != -1 {
                //let verts_edited = utils::bone_meshes_edited(
                //    self.textures[tex_idx as usize].size,
                //    &self.find_bone(bone_id).unwrap().vertices,
                //);
                //if !verts_edited {
                //}
            }
        } else {
            // record texture change in animation
            let kf = self.animations[selected_anim].check_if_in_keyframe(
                bone_id as i32,
                selected_frame,
                AnimElement::TextureIndex,
                -1,
            );
            self.animations[selected_anim].keyframes[kf].value = new_tex_idx as f32;

            // add 0th keyframe
            let first = self.animations[selected_anim].check_if_in_keyframe(
                bone_id as i32,
                0,
                AnimElement::TextureIndex,
                -1,
            );
            self.animations[selected_anim].keyframes[first].value = tex_idx as f32;
        }

        if tex_set_idx == -1
            || new_tex_idx > self.texture_sets[tex_set_idx as usize].textures.len() - 1
        {
            return;
        }

        let name = self.texture_sets[tex_set_idx as usize].textures[new_tex_idx]
            .name
            .clone();
        let bone_name = &mut self.find_bone_mut(bone_id).unwrap().name;
        if bone_name == NEW_BONE_NAME || bone_name == "" {
            *bone_name = name;
        }

        if self.find_bone(bone_id).unwrap().tex_idx == -1 {
            self.find_bone_mut(bone_id).unwrap().tex_idx = 0;
        }

        (
            self.find_bone_mut(bone_id).unwrap().vertices,
            self.find_bone_mut(bone_id).unwrap().indices,
        ) = renderer::create_tex_rect(
            &self.texture_sets[tex_set_idx as usize].textures[new_tex_idx].size,
        );
    }

    pub fn delete_bone(&mut self, id: i32) {
        for i in 0..self.bones.len() {
            let bone_id = self.bones[i].id;
            if bone_id == id {
                self.bones.remove(i);
                break;
            }
        }
    }

    pub fn new_bone(&mut self, id: i32) -> (Bone, usize) {
        let mut parent_id = -1;
        if self.find_bone(id) != None {
            parent_id = self.find_bone(id).unwrap().parent_id;
        }
        let ids = self.bones.iter().map(|a| a.id).collect();
        let new_bone = Bone {
            name: NEW_BONE_NAME.to_string(),
            parent_id,
            id: generate_id(ids),
            scale: Vec2 { x: 1., y: 1. },
            tex_set_idx: -1,
            pivot: Vec2::new(0.5, 0.5),
            zindex: self.bones.len() as f32,
            constraint: JointConstraint::None,
            ..Default::default()
        };
        if id == -1 {
            self.bones.push(new_bone.clone());
        } else {
            // add new bone below targeted one, keeping in mind its children
            for i in 0..self.bones.len() {
                if self.bones[i].id != id {
                    continue;
                }

                let mut children = vec![];
                crate::armature_window::get_all_children(
                    &self.bones,
                    &mut children,
                    &self.bones[i],
                );
                let idx = i + children.len() + 1;
                self.bones.insert(idx, new_bone.clone());
                return (new_bone, idx);
            }
        }
        (new_bone, self.bones.len() - 1)
    }

    // generate non-clashing id
    pub fn generate_id(&self) -> i32 {
        let mut idx = 0;
        while idx == self.does_id_exist(idx) {
            idx += 1;
        }
        return idx;
    }

    pub fn does_id_exist(&self, id: i32) -> i32 {
        for b in &self.bones {
            if b.id == id {
                return id;
            }
        }
        return -1;
    }

    pub fn edit_bone(
        &mut self,
        bone_id: i32,
        element: &AnimElement,
        mut value: f32,
        anim_id: usize,
        anim_frame: i32,
    ) {
        macro_rules! edit {
            ($field:expr) => {
                if anim_id == usize::MAX {
                    $field = value;
                } else {
                    // offset value by its field, so it's effectively overwritten
                    match element {
                        AnimElement::ScaleX | AnimElement::ScaleY => value /= $field,
                        _ => value -= $field,
                    }
                }
            };
        }

        let bone_mut = self.find_bone_mut(bone_id).unwrap();

        #[rustfmt::skip]
        match element {
            AnimElement::PositionX     => { edit!(bone_mut.pos.x);   },
            AnimElement::PositionY     => { edit!(bone_mut.pos.y);   },
            AnimElement::Rotation      => { edit!(bone_mut.rot);     },
            AnimElement::ScaleX        => { edit!(bone_mut.scale.x); },
            AnimElement::ScaleY        => { edit!(bone_mut.scale.y); },
            AnimElement::PivotX        => { edit!(bone_mut.pivot.x); },
            AnimElement::PivotY        => { edit!(bone_mut.pivot.y); },
            AnimElement::Zindex        => { edit!(bone_mut.zindex);  },
            AnimElement::VertPositionX => { /* do nothing */ },
            AnimElement::VertPositionY => { /* do nothing */ },
            AnimElement::TextureIndex       => { /* handled in set_bone_tex() */ },
        };

        if anim_id == usize::MAX {
            return;
        }

        // create keyframe at 0th frame for this element if it doesn't exist
        if anim_frame != 0 {
            let frame =
                self.animations[anim_id].check_if_in_keyframe(bone_id, 0, element.clone(), -1);
            self.animations[anim_id].keyframes[frame].value = match element {
                AnimElement::ScaleX | AnimElement::ScaleY => 1.,
                _ => 0.,
            }
        }
        let frame =
            self.animations[anim_id].check_if_in_keyframe(bone_id, anim_frame, element.clone(), -1);
        self.animations[anim_id].keyframes[frame].value = value;
    }

    pub fn edit_vert(
        &mut self,
        bone_id: i32,
        vert_id: i32,
        pos: &Vec2,
        anim_id: usize,
        anim_frame: i32,
    ) {
        let bone_id = self.bones[bone_id as usize].id;

        macro_rules! animate {
            ($element:expr, $value:expr) => {
                // create 0th frame
                let first_frame =
                    self.animations[anim_id].check_if_in_keyframe(bone_id, 0, $element, vert_id);
                self.animations[anim_id].keyframes[first_frame].vert_id = vert_id as i32;

                let frame = self.animations[anim_id]
                    .check_if_in_keyframe(bone_id, anim_frame, $element, vert_id);
                self.animations[anim_id].keyframes[frame].value = $value;
                self.animations[anim_id].keyframes[frame].vert_id = vert_id as i32;
            };
        }

        animate!(AnimElement::VertPositionX, pos.x);
        animate!(AnimElement::VertPositionY, pos.y);
    }

    // runtime: core animation logic
    pub fn animate(
        &mut self,
        anim_idx: usize,
        anim_frame: i32,
        og_bones: Option<&Vec<Bone>>,
    ) -> Vec<Bone> {
        let mut bones = if og_bones != None {
            og_bones.unwrap().clone()
        } else {
            self.bones.clone()
        };

        // ignore if this animation has no keyframes
        let kf_len = self.animations[anim_idx].keyframes.len();
        if kf_len == 0 {
            return bones;
        }

        for b in &mut bones {
            if self.is_bone_hidden(b.id) {
                continue;
            }

            macro_rules! interpolate {
                ($element:expr, $default:expr, $vert_id:expr) => {{
                    let (prev, next, total_frames, current_frame, transition) = self
                        .find_connecting_frames(
                            anim_idx, b.id, $vert_id, $element, $default, anim_frame,
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

            macro_rules! prev_frame {
                ($element:expr, $default:expr) => {
                    self.find_connecting_frames(anim_idx, b.id, -1, $element, $default, anim_frame)
                        .0
                };
            }

            // iterable anim interps
            #[rustfmt::skip]
            {
                b.pos.x   += interpolate!(AnimElement::PositionX,    0., -1);
                b.pos.y   += interpolate!(AnimElement::PositionY,    0., -1);
                b.rot     += interpolate!(AnimElement::Rotation,     0., -1);
                b.scale.x *= interpolate!(AnimElement::ScaleX,       1., -1);
                b.scale.y *= interpolate!(AnimElement::ScaleY,       1., -1);
                b.pivot.x += interpolate!(AnimElement::PivotX,       0., -1);
                b.pivot.y += interpolate!(AnimElement::PivotY,       0., -1);
                b.zindex  =  prev_frame!( AnimElement::Zindex,       0.);
                b.tex_idx =  prev_frame!( AnimElement::TextureIndex, b.tex_idx as f32) as i32;
            };

            // restructure bone's verts to match texture
            if b.tex_set_idx != -1 {
                let set = &self.texture_sets[self.find_bone(b.id).unwrap().tex_set_idx as usize];
                let set_tex_limit = self.texture_sets[b.tex_set_idx as usize].textures.len() - 1;
                if b.tex_idx != -1 && b.tex_set_idx < set_tex_limit as i32 {
                    (
                        self.find_bone_mut(b.id).unwrap().vertices,
                        self.find_bone_mut(b.id).unwrap().indices,
                    ) = renderer::create_tex_rect(&set.textures[b.tex_idx as usize].size);
                }
            }

            for v in 0..b.vertices.len() {
                b.vertices[v].pos.x += interpolate!(AnimElement::VertPositionX, 0., v as i32);
                b.vertices[v].pos.y += interpolate!(AnimElement::VertPositionY, 0., v as i32);
            }
        }

        bones
    }

    pub fn find_connecting_frames(
        &self,
        anim_id: usize,
        bone_id: i32,
        vert_id: i32,
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
        let keyframes = &self.animations[anim_id].keyframes;
        for (i, kf) in keyframes.iter().enumerate() {
            if self.animations[anim_id].keyframes[i].frame > frame {
                break;
            }

            if kf.bone_id != bone_id || kf.element != element || kf.vert_id != vert_id {
                continue;
            }

            prev = Some(kf.value);
            start_frame = kf.frame;
        }

        // get first next frame with this element
        for (i, kf) in keyframes.iter().enumerate().rev() {
            if self.animations[anim_id].keyframes[i].frame < frame {
                break;
            }

            if kf.bone_id != bone_id || kf.element != element || kf.vert_id != vert_id {
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

    /// unfold this bone's parents so it can be seen in the armature window
    pub fn unfold_to_bone(&mut self, bone_id: i32) {
        let parents = self.get_all_parents(bone_id);
        for parent in &parents {
            self.find_bone_mut(parent.id).unwrap().folded = false;
        }
    }

    pub fn get_all_parents(&self, bone_id: i32) -> Vec<Bone> {
        // add own bone temporarily
        let mut parents: Vec<Bone> = vec![self.find_bone(bone_id).unwrap().clone()];

        while parents.last().unwrap().parent_id != -1 {
            parents.push(
                self.find_bone(parents.last().unwrap().parent_id)
                    .unwrap()
                    .clone(),
            );
        }

        // remove own bone from list
        parents.remove(0);

        parents
    }

    pub fn offset_bone_by_parent(&mut self, old_parents: Vec<Bone>, bone_id: i32) {
        for parent in old_parents {
            let parent_pos = parent.pos;
            self.find_bone_mut(bone_id).unwrap().pos += parent_pos;
        }

        if self.find_bone_mut(bone_id).unwrap().parent_id == -1 {
            return;
        }

        let new_parents = self.get_all_parents(bone_id);

        for parent in new_parents {
            let parent_pos = parent.pos;
            self.find_bone_mut(bone_id).unwrap().pos -= parent_pos;
        }
    }

    pub fn new_animation(&mut self) {
        let ids = self.animations.iter().map(|a| a.id).collect();
        self.animations.push(Animation {
            name: "".to_string(),
            id: generate_id(ids),
            keyframes: vec![],
            fps: 60,
            ..Default::default()
        });
    }

    pub fn is_bone_hidden(&self, bone_id: i32) -> bool {
        if self.find_bone(bone_id) == None {
            return false;
        }

        if self.find_bone(bone_id).unwrap().hidden {
            return true;
        }

        let parents = self.get_all_parents(bone_id);

        for parent in &parents {
            if parent.hidden {
                return true;
            }
        }

        false
    }
}

// used for the json
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Root {
    pub version: String,
    pub texture_size: Vec2,
    pub armature: Armature,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq)]
pub struct TextureSet {
    pub name: String,
    #[serde(default)]
    pub textures: Vec<Texture>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq)]
pub struct Texture {
    #[serde(default)]
    pub offset: Vec2,
    #[serde(default)]
    pub size: Vec2,
    #[serde(default)]
    pub name: String,
    #[serde(skip)]
    pub image: image::DynamicImage,
    #[serde(skip)]
    pub bind_group: Option<BindGroup>,
    #[serde(skip)]
    pub ui_img: Option<egui::TextureHandle>,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Animation {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub fps: i32,
    #[serde(default)]
    pub keyframes: Vec<Keyframe>,
    #[serde(skip)]
    pub elapsed: Option<Instant>,
}

impl Animation {
    /// Return which frame has these attributes, or create a new one
    pub fn check_if_in_keyframe(
        &mut self,
        id: i32,
        frame: i32,
        element: AnimElement,
        vert_id: i32,
    ) -> usize {
        macro_rules! is_same_frame {
            ($kf:expr) => {
                $kf.frame == frame
                    && $kf.bone_id == id
                    && $kf.element == element
                    && $kf.vert_id == vert_id
            };
        }

        // check if this keyframe exists
        let mut exists_at = usize::MAX;
        for i in 0..self.keyframes.len() {
            let kf = &self.keyframes[i];
            if is_same_frame!(kf) {
                exists_at = i;
                break;
            }
        }

        if exists_at != usize::MAX {
            return exists_at;
        }

        self.keyframes.push(Keyframe {
            frame,
            bone_id: id,
            element: element.clone(),
            element_id: element.clone() as i32,
            vert_id,
            ..Default::default()
        });

        self.sort_keyframes();

        for i in 0..self.keyframes.len() {
            let kf = &self.keyframes[i];
            if is_same_frame!(kf) {
                return i;
            }
        }

        usize::MAX
    }

    pub fn sort_keyframes(&mut self) {
        self.keyframes.sort_by(|a, b| a.frame.cmp(&b.frame));
    }

    pub fn remove_all_keyframes_of_frame(&mut self, frame: i32) {
        for k in (0..self.keyframes.len()).rev() {
            let kf = &self.keyframes[k];
            if kf.frame == frame {
                self.keyframes.remove(k);
            }
        }
    }

    pub fn get_frame(&self) -> i32 {
        if self.elapsed == None || self.keyframes.len() == 0 {
            return 0;
        }

        let elapsed = self.elapsed.unwrap().elapsed().as_millis() as f32 / 1e3 as f32;
        let frametime = 1. / self.fps as f32;

        // Offset elapsed time with the selected frame.
        // This only applies for the first play cycle, since selected frame
        // is reset on the next one.
        // elapsed += shared.ui.anim.played_frame as f32 * frametime;

        (elapsed / frametime) as i32
    }

    pub fn set_frame(&mut self) -> i32 {
        if self.elapsed == None || self.keyframes.len() == 0 {
            return 0;
        }

        let mut frame = self.get_frame();

        if frame >= self.keyframes.last().unwrap().frame {
            self.elapsed = Some(Instant::now());
            frame = 0;
        }

        frame
    }
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Keyframe {
    #[serde(default)]
    pub frame: i32,
    #[serde(default)]
    pub bone_id: i32,
    #[serde(default)]
    pub element_id: i32,
    #[serde(default = "default_neg_one", skip_serializing_if = "is_neg_one")]
    pub vert_id: i32,
    #[serde(default)]
    pub element: AnimElement,

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
    VertPositionX,
    VertPositionY,
    TextureIndex,
}

// iterable anim change icons IDs
#[rustfmt::skip]
pub const ANIM_ICON_ID: [usize; 10] = [
    0,
    0,
    1,
    2,
    2,
    3,
    3,
    0,
    0,
    4,
];

#[derive(Default, Clone, PartialEq)]
pub enum ActionEnum {
    #[default]
    Bone,
    Animation,
    Keyframe,
    Bones,
    Animations,
}

#[derive(Default, Clone, PartialEq)]
pub struct Action {
    pub action: ActionEnum,

    pub id: i32,
    pub bones: Vec<Bone>,
    pub animations: Vec<Animation>,
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
    pub fn find(&self, id: i32, element: &AnimElement, vert_id: i32) -> Option<&BoneTop> {
        for bt in &self.tops {
            if bt.id == id && bt.element == *element && bt.vert_id == vert_id {
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
    pub vert_id: i32,
    pub element: AnimElement,
    pub height: f32,
}
#[derive(Clone)]
pub struct RenderedFrame {
    pub buffer: wgpu::Buffer,
    pub width: u32,
    pub height: u32,
}

#[derive(Default, PartialEq, Clone, Debug)]
pub enum TutorialStep {
    NewBone,
    GetImage,
    EditBoneX,
    EditBoneY,
    OpenAnim,
    CreateAnim,
    SelectKeyframe,
    EditBoneAnim,
    PlayAnim,
    StopAnim,
    Finish,

    // tutorial is meant to work with first bone only,
    // so it must be reselected to proceed
    ReselectBone,

    #[default]
    None,
}

enum_string!(TutorialStep);

#[derive(Default)]
pub struct CopyBuffer {
    pub keyframes: Vec<Keyframe>,
}

#[derive(Default, Clone)]
pub struct TempPath {
    pub base: String,
    pub img: String,
    pub save: String,
    pub import: String,
    pub export_vid_text: String,
    pub export_vid_done: String,
}

#[derive(Default, Clone, PartialEq)]
pub enum Saving {
    #[default]
    None,
    CustomPath,
    Autosaving,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct StartupResourceItem {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url_type: StartupItemType,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub is_dev: bool,
    #[serde(default)]
    pub items: Vec<StartupResourceItem>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, PartialEq)]
pub enum StartupItemType {
    #[default]
    Custom,
    DevDocs,
    UserDocs,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Startup {
    #[serde(default)]
    pub resources: Vec<StartupResourceItem>,
}

#[derive(Default)]
pub struct Shared {
    pub window: Vec2,
    pub window_factor: f32,
    pub armature: Armature,
    pub camera: Camera,
    pub input: InputStates,
    pub cursor_icon: egui::CursorIcon,
    pub ui: Ui,
    pub editing_bone: bool,

    pub dragging_vert: usize,

    pub frame: i32,
    pub recording: bool,
    pub done_recording: bool,
    // mainly used for video, but can also be used for screenshots
    pub rendered_frames: Vec<RenderedFrame>,

    pub undo_actions: Vec<Action>,
    pub redo_actions: Vec<Action>,

    pub edit_mode: EditMode,

    pub generic_bindgroup: Option<BindGroup>,

    pub save_path: String,

    pub recent_file_paths: Vec<String>,

    pub temp_path: TempPath,
    pub has_temp: bool,

    pub config: Config,

    pub copy_buffer: CopyBuffer,

    pub gridline_gap: i32,

    pub saving: Saving,

    pub thumb_ui_tex: std::collections::HashMap<String, egui::TextureHandle>,

    pub startup: Startup,

    /// triggers debug stuff. Set in main.rs
    pub debug: bool,

    pub time: f32,

    pub last_autosave: f32,
}

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

    pub fn last_keyframe(&self) -> Option<&Keyframe> {
        self.selected_animation().unwrap().keyframes.last()
    }

    pub fn selected_bone(&self) -> Option<&Bone> {
        if self.ui.selected_bone_idx != usize::MAX {
            return Some(&self.armature.bones[self.ui.selected_bone_idx]);
        }
        None
    }

    pub fn selected_bone_mut(&mut self) -> Option<&mut Bone> {
        if self.ui.selected_bone_idx != usize::MAX {
            return Some(&mut self.armature.bones[self.ui.selected_bone_idx]);
        }
        None
    }

    pub fn save_edited_bone(&mut self) {
        if self.ui.is_animating() {
            self.undo_actions.push(Action {
                action: ActionEnum::Animation,
                id: self.selected_animation().unwrap().id as i32,
                animations: vec![self.selected_animation().unwrap().clone()],
                ..Default::default()
            });
        } else {
            self.undo_actions.push(Action {
                action: ActionEnum::Bone,
                id: self.selected_bone().unwrap().id,
                bones: vec![self.selected_bone().unwrap().clone()],
                ..Default::default()
            });
        }
    }

    pub fn remove_texture(&mut self, set_idx: i32, tex_idx: i32) {
        self.armature.texture_sets[set_idx as usize]
            .textures
            .remove(tex_idx as usize);
        //self.armature.bind_groups.remove(tex_idx as usize);
        for bone in &mut self.armature.bones {
            if bone.tex_idx == tex_idx {
                bone.tex_idx = -1;
            }
            if bone.tex_idx > tex_idx {
                bone.tex_idx -= 1;
            }
        }
    }

    pub fn mouse_vel(&self) -> Vec2 {
        let mouse_world = utils::screen_to_world_space(self.input.mouse, self.window);
        let mouse_prev_world = utils::screen_to_world_space(self.input.mouse_prev, self.window);
        mouse_prev_world - mouse_world
    }
}

// generate non-clashing id
pub fn generate_id(ids: Vec<i32>) -> i32 {
    let mut idx = 0;
    while idx == does_id_exist(idx, ids.clone()) {
        idx += 1;
    }
    return idx;
}

pub fn does_id_exist(id: i32, ids: Vec<i32>) -> i32 {
    for this_id in ids {
        if this_id == id {
            return id;
        }
    }
    return -1;
}

// serde stuff

fn default_neg_one() -> i32 {
    -1
}

fn default_one() -> f32 {
    1.
}

fn gridline_default() -> i32 {
    200
}

fn is_neg_one<T: std::cmp::PartialEq<i32>>(value: &T) -> bool {
    *value == -1
}

fn are_verts_empty<T: std::cmp::PartialEq<Vec<Vertex>>>(value: &T) -> bool {
    *value == vec![]
}

fn are_indices_empty<T: std::cmp::PartialEq<Vec<u32>>>(value: &T) -> bool {
    *value == vec![]
}

fn are_anims_empty<T: std::cmp::PartialEq<Vec<Animation>>>(value: &T) -> bool {
    *value == vec![]
}

#[cfg(not(target_arch = "wasm32"))]
pub fn config_path() -> std::path::PathBuf {
    directories_next::ProjectDirs::from("com", "retropaint", "skelform")
        .map(|proj_dirs| proj_dirs.data_dir().join("config.json"))
        .unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn recents_path() -> std::path::PathBuf {
    directories_next::ProjectDirs::from("com", "retropaint", "skelform")
        .map(|proj_dirs| proj_dirs.data_dir().join("recent_files.json"))
        .unwrap()
}
