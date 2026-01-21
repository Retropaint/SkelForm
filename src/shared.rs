//! Easily-accessible and frequently-shared data.

use crate::*;

use std::{
    fmt,
    ops::{DivAssign, MulAssign},
};

use std::sync::Mutex;
use wgpu::BindGroup;

#[rustfmt::skip]
#[repr(C)]
#[derive(Debug,serde::Serialize,serde::Deserialize,Default,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable)]
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

impl From<(u32, u32)> for Vec2 {
    fn from(pos: (u32, u32)) -> Vec2 {
        Vec2::new(pos.0 as f32, pos.1 as f32)
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
        if mag == 0. {
            return Vec2::default();
        }
        Vec2::new(self.x / mag, self.y / mag)
    }

    pub fn floor(&self) -> Self {
        Vec2::new(self.x.floor(), self.y.floor())
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

        let x = (self.x * dp).trunc() / dp;
        let y = (self.y * dp).trunc() / dp;
        write!(f, "{}, {}", x, y)
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

// Higher-level vertex data, used for the SkF format.
// See GpuVertex for actual vertex data supplied to wgpu
#[rustfmt::skip]
#[repr(C)]
#[derive(PartialEq,serde::Serialize,serde::Deserialize,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable,Debug)]
pub struct Vertex {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub pos: Vec2,
    #[serde(default)]
    pub uv: Vec2,
    #[serde(default)] 
    pub init_pos: Vec2,
    #[serde(skip)]
    pub color: VertexColor,
    #[serde(skip)]
    pub add_color: VertexColor,
    #[serde(skip)]
    pub offset_rot: f32,
}

// The vertex data supplied to wgpu.
#[rustfmt::skip]
#[repr(C)]
#[derive(PartialEq,serde::Serialize,serde::Deserialize,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable,Debug)]
pub struct GpuVertex {
    #[serde(default)]
    pub pos: Vec2,
    #[serde(default)]
    pub uv: Vec2,
    #[serde(skip)]
    pub color: VertexColor,
    #[serde(skip)]
    pub add_color: VertexColor,
}

impl Default for Vertex {
    fn default() -> Self {
        Vertex {
            id: 0,
            pos: Vec2::default(),
            uv: Vec2::default(),
            init_pos: Vec2::default(),
            color: VertexColor::default(),
            add_color: VertexColor::new(0., 0., 0., 0.),
            offset_rot: 0.,
        }
    }
}

impl From<Vertex> for GpuVertex {
    fn from(vert: Vertex) -> GpuVertex {
        GpuVertex {
            pos: vert.pos,
            uv: vert.uv,
            color: vert.color,
            add_color: vert.add_color,
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
    pub const YELLOW: VertexColor = VertexColor::new(1., 1., 0., 1.);
    pub const WHITE: VertexColor = VertexColor::new(1., 1., 1., 1.);
}

impl std::ops::AddAssign for VertexColor {
    fn add_assign(&mut self, other: VertexColor) {
        self.r += other.r;
        self.g += other.g;
        self.b += other.b;
        self.a += other.a;
    }
}

#[rustfmt::skip]
impl Default for VertexColor {
    fn default() -> Self {
        VertexColor {  r: 1., g: 1., b: 1., a: 1. }
    }
}

#[rustfmt::skip]
#[repr(C)]
#[derive(PartialEq,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable,Debug,serde::Deserialize,serde::Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(skip)]
    pub a: u8,
}

impl std::ops::AddAssign for Color {
    fn add_assign(&mut self, other: Color) {
        macro_rules! add {
            ($col:expr, $other_col:expr) => {
                $col = if let Some(col) = $col.checked_add($other_col) {
                    col
                } else {
                    255
                }
            };
        }
        add!(self.r, other.r);
        add!(self.g, other.g);
        add!(self.b, other.b);
        add!(self.a, other.a);
    }
}

impl std::ops::SubAssign for Color {
    fn sub_assign(&mut self, other: Color) {
        macro_rules! sub {
            ($col:expr, $other_col:expr) => {
                $col = if let Some(col) = $col.checked_sub($other_col) {
                    col
                } else {
                    0
                }
            };
        }
        sub!(self.r, other.r);
        sub!(self.g, other.g);
        sub!(self.b, other.b);
        sub!(self.a, other.a);
    }
}

impl std::ops::Add for Color {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
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
#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct Camera {
    pub pos: Vec2,
    pub zoom: f32,
}

/// Input-related fields.
#[derive(Default)]
pub struct InputStates {
    pub mouse: Vec2,
    pub mouse_prev: Vec2,
    pub left_clicked: bool,
    pub left_pressed: bool,
    pub right_clicked: bool,
    pub left_down: bool,
    pub right_down: bool,
    pub down_dur: i32,
    pub holding_mod: bool,
    pub holding_shift: bool,
    pub mouse_init: Option<Vec2>,
    pub scroll_delta: f32,

    pub mod_q: Option<global_hotkey::hotkey::HotKey>,
    pub mod_w: Option<global_hotkey::hotkey::HotKey>,
    pub hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,

    // is mouse on UI?
    pub on_ui: bool,

    pub last_pressed: Option<egui::Key>,
}

#[derive(Clone, Default, PartialEq)]
pub enum UiState {
    #[default]
    StylesModal,
    Exiting,
    DraggingBone,
    RemovingTexture,
    ForcedModal,
    Modal,
    PolarModal,
    SettingsModal,
    StartupWindow,
    Scaling,
    Rotating,
    FocusStyleDropdown,
}

#[derive(Clone, Default, PartialEq)]
pub enum SettingsState {
    #[default]
    Ui,
    Animation,
    Rendering,
    Keyboard,
    Misc,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub enum PolarId {
    #[default]
    DeleteBone,
    Exiting,
    DeleteAnim,
    DeleteFile,
    DeleteTex,
    DeleteStyle,
    NewUpdate,
}
enum_string!(PolarId);

#[derive(Clone, Default)]
pub struct ContextMenu {
    pub id: String,
    pub hide: bool,
    pub keep: bool,
}

impl ContextMenu {
    pub fn show(&mut self, id: &String) {
        self.id = id.clone();
        self.hide = false;
    }

    pub fn close(&mut self) {
        self.id = "".to_string();
        self.keep = false;
    }

    pub fn is(&self, id: &String) -> bool {
        self.id == *id && !self.hide
    }
}

#[derive(Clone, Default)]
pub struct UiBar {
    pub pos: Vec2,
    pub scale: Vec2,
}

#[derive(Clone, Default)]
pub struct Ui {
    pub anim: UiAnim,
    pub startup: Startup,

    pub edit_bar: UiBar,
    pub anim_bar: UiBar,
    pub camera_bar: UiBar,

    pub bone_panel_rect: Option<egui::Rect>,
    pub armature_panel_rect: Option<egui::Rect>,
    pub keyframe_panel_rect: Option<egui::Rect>,
    pub top_panel_rect: Option<egui::Rect>,

    pub selected_bone_idx: usize,
    pub selected_bone_ids: Vec<i32>,
    pub showing_mesh: bool,
    pub setting_bind_verts: bool,
    pub setting_bind_bone: bool,

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

    // context menu stuff

    // determines if context menu should close on next click
    pub context_menu: ContextMenu,

    pub settings_state: SettingsState,

    pub changing_key: String,

    pub selected_style: i32,

    pub hovering_tex: i32,
    pub hovering_bone: i32,
    pub hovering_set: i32,
    pub hovering_anim: i32,
    pub hovering_style_bone: i32,
    pub hovering_setting: Option<shared::SettingsState>,

    pub showing_samples: bool,

    pub selected_path: String,

    pub setting_ik_target: bool,

    // not visually indicated; just used for `double click > rename` logic
    pub selected_tex: i32,

    pub selected_bind: i32,

    pub local_doc_url: String,

    pub mobile: bool,
    pub was_editing_path: bool,
    pub thumb_ui_tex: std::collections::HashMap<String, egui::TextureHandle>,

    pub styles_modal_size: Vec2,
    pub bones_assigned_scroll: f32,
    pub dragging_tex: bool,

    pub pending_textures: Vec<Texture>,
    pub done_pending: bool,
    pub init_pending_mouse: Vec2,
    pub is_dragging_pending: bool,
    pub prev_pending_interp: Vec2,
    pub just_made_bone: bool,
    pub just_made_anim: bool,
    pub just_made_style: bool,

    // states
    pub styles_modal: bool,
    pub exiting: bool,
    pub confirmed_exit: bool,
    pub dragging_bone: bool,
    pub removing_textures: bool,
    pub forced_modal: bool,
    pub modal: bool,
    pub polar_modal: bool,
    pub settings_modal: bool,
    pub startup_window: bool,
    pub scaling: bool,
    pub rotating: bool,
    pub focus_style_dropdown: bool,
    pub donating_modal: bool,
    pub atlas_modal: bool,
    pub checking_update: bool,
    pub update_request_sent: bool,
    pub new_version: String,

    loc_strings: std::collections::HashMap<String, String>,
    pub cursor_icon: egui::CursorIcon,
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

    pub fn open_modal(&mut self, headline: String, forced: bool) {
        self.modal = true;
        self.forced_modal = forced;
        self.headline = headline;
    }

    pub fn open_polar_modal(&mut self, id: PolarId, headline: String) {
        self.polar_modal = true;
        self.polar_id = id;
        self.headline = headline.to_string();
    }

    pub fn unselect_everything(&mut self) {
        self.selected_bone_idx = usize::MAX;
        self.selected_bone_ids = vec![];
        self.anim.selected_frame = -1;
        self.showing_mesh = false;
        self.anim.selected = usize::MAX;
        self.selected_bind = -1;
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

    pub fn select_bone(&mut self, idx: usize) {
        let selected_anim = self.anim.selected;
        self.unselect_everything();
        self.anim.selected = selected_anim;
        self.selected_bone_idx = idx;
        self.setting_bind_verts = false;
        self.setting_bind_bone = false;
        self.selected_bind = -1;
        self.rename_id = "".to_string();
    }

    pub fn context_id_parsed(&self) -> i32 {
        let raw_id = self.context_menu.id.split('_').collect::<Vec<_>>()[1];
        raw_id.parse::<i32>().unwrap()
    }

    /// Localization
    /// Extracts the specified text from the current language.
    /// ex: `settings_modal.user_interface.general` -> "General"
    ///
    /// All localized text *must* be from this method, as edge cases and fallbacks must be handled as well.
    pub fn loc(&self, str: &str) -> String {
        let result = self.loc_strings.get(str);
        if let Some(string) = result {
            return string.to_string();
        }

        self.loc_strings[""].to_string()
    }
    pub fn init_empty_loc(&mut self) {
        self.loc_strings.insert("".to_string(), "".to_string());
    }

    pub fn init_lang(&mut self, lang_json: serde_json::Value) {
        utils::flatten_json(
            &lang_json,
            "".to_string(),
            &mut self.loc_strings,
            "".to_string(),
        );
        self.loc_strings.insert("".to_string(), "".to_string());
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Eq, Debug, Clone)]
pub enum UiLayout {
    #[default]
    Split,
    Right,
    Left,
}

enum_string!(UiLayout);

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Config {
    #[serde(default = "default_one")]
    pub ui_scale: f32,
    #[serde(default = "gridline_default")]
    pub gridline_gap: i32,
    #[serde(default)]
    pub skip_startup: bool,
    #[serde(default)]
    pub autosave_frequency: i32,
    #[serde(default)]
    pub exact_bone_select: bool,
    #[serde(default)]
    pub gridline_front: bool,
    #[serde(default)]
    pub keep_tex_str: bool,
    #[serde(default)]
    pub edit_while_playing: bool,
    #[serde(default)]
    pub layout: UiLayout,
    #[serde(default)]
    pub ignore_donate: bool,

    #[serde(default)]
    pub colors: ColorConfig,
    #[serde(default)]
    pub keys: KeyboardConfig,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ColorConfig {
    pub main: Color,
    pub light_accent: Color,
    pub dark_accent: Color,
    pub text: Color,
    pub frameline: Color,
    pub gradient: Color,
    pub background: Color,
    pub gridline: Color,
    pub center_point: Color,
    pub link: Color,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ui_scale: default_one(),
            colors: ColorConfig::default(),
            keys: KeyboardConfig::default(),
            gridline_gap: gridline_default(),
            skip_startup: false,
            autosave_frequency: 5,
            exact_bone_select: false,
            gridline_front: false,
            keep_tex_str: false,
            layout: UiLayout::Split,
            edit_while_playing: false,
            ignore_donate: false,
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
            gridline: Color::new(128, 128, 128, 255),
            center_point: Color::new(0, 255, 0, 255),
            link: Color::new(193, 165, 221, 255),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
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
    pub copy: egui::KeyboardShortcut,
    pub paste: egui::KeyboardShortcut,
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

macro_rules! shortcut_key {
    ($mod:expr, $key:expr) => {
        egui::KeyboardShortcut::new($mod, $key)
    };
}

impl Default for KeyboardConfig {
    #[rustfmt::skip]
    fn default() -> Self {
        KeyboardConfig {
            next_anim_frame: regular_key!(egui::Key::ArrowRight),
            prev_anim_frame: regular_key!(egui::Key::ArrowLeft),
            zoom_in_camera:  regular_key!(egui::Key::Equals),
            zoom_out_camera: regular_key!(egui::Key::Minus),
            cancel:          regular_key!(egui::Key::Escape),
            undo:            shortcut_key!(egui::Modifiers::COMMAND, egui::Key::Z),
            redo:            shortcut_key!(egui::Modifiers::COMMAND, egui::Key::Y),
            save:            shortcut_key!(egui::Modifiers::COMMAND, egui::Key::S),
            open:            shortcut_key!(egui::Modifiers::COMMAND, egui::Key::O),
            copy:            shortcut_key!(egui::Modifiers::COMMAND, egui::Key::C),
            paste:           shortcut_key!(egui::Modifiers::COMMAND, egui::Key::V),
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

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Default, PartialEq, Debug)]
pub enum JointConstraint {
    #[default]
    None,
    Clockwise,
    CounterClockwise,
    Skip,
}

enum_string!(JointConstraint);

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default, Debug)]
pub struct Bone {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub parent_id: i32,
    #[serde(default, skip_serializing_if = "is_str_empty")]
    pub tex: String,
    #[serde(default, skip_serializing_if = "is_neg_one")]
    pub zindex: i32,
    #[serde(default)]
    pub pos: Vec2,
    #[serde(default)]
    pub scale: Vec2,
    #[serde(default)]
    pub rot: f32,
    #[serde(default)]
    pub is_hidden: bool,

    #[serde(default = "default_neg_one")]
    pub ik_family_id: i32,
    #[rustfmt::skip]
    #[serde(default, skip_serializing_if = "no_constraints", rename = "ik_constraint_str")]
    pub ik_constraint: JointConstraint,
    #[serde(default, skip_serializing_if = "is_neg_one", rename = "ik_constraint")]
    pub ik_constraint_id: i32,
    #[serde(default, skip_serializing_if = "no_ik_mode", rename = "ik_mode_str")]
    pub ik_mode: InverseKinematicsMode,
    #[serde(default, skip_serializing_if = "is_neg_one", rename = "ik_mode")]
    pub ik_mode_id: i32,
    #[serde(default = "default_neg_one", skip_serializing_if = "is_neg_one")]
    pub ik_target_id: i32,
    #[serde(default, skip_serializing_if = "is_i32_empty")]
    pub ik_bone_ids: Vec<i32>,

    // todo:
    // these should be private, but that upsets
    // default constructor for some reason
    #[serde(default, skip_deserializing)]
    pub init_pos: Vec2,
    #[serde(default, skip_deserializing)]
    pub init_scale: Vec2,
    #[serde(default, skip_deserializing)]
    pub init_rot: f32,
    #[serde(default, skip_serializing_if = "is_neg_one", skip_deserializing)]
    pub init_ik_constraint: i32,
    #[serde(default, skip_serializing_if = "is_false", skip_deserializing)]
    pub init_is_hidden: bool,
    #[serde(default, skip_serializing_if = "is_str_empty", skip_deserializing)]
    pub init_tex: String,

    #[serde(default, skip_serializing_if = "are_verts_empty")]
    pub vertices: Vec<Vertex>,
    #[serde(skip)]
    pub verts_edited: bool,
    #[serde(default, skip_serializing_if = "are_indices_empty")]
    pub indices: Vec<u32>,
    #[serde(default, skip_serializing_if = "are_weights_empty")]
    pub binds: Vec<BoneBind>,

    #[serde(skip)]
    pub folded: bool,
    #[serde(skip)]
    pub ik_folded: bool,
    #[serde(skip)]
    pub meshdef_folded: bool,
    #[serde(skip)]
    pub world_verts: Vec<Vertex>,
    #[serde(skip)]
    pub ik_disabled: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default, Debug)]
pub struct BoneBind {
    #[serde(default = "default_neg_one")]
    pub bone_id: i32,
    #[serde(default)]
    pub is_path: bool,
    #[serde(default)]
    pub verts: Vec<BoneBindVert>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default, Debug)]
pub struct BoneBindVert {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub weight: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct EditorStyle {
    #[serde(default)]
    pub active: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct EditorOptions {
    #[serde(default)]
    pub camera: Camera,
    #[serde(default)]
    pub bones: Vec<EditorBone>,
    #[serde(default)]
    pub styles: Vec<EditorStyle>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct EditorBone {
    #[serde(default)]
    pub folded: bool,
    #[serde(default)]
    pub ik_folded: bool,
    #[serde(default)]
    pub meshdef_folded: bool,
    #[serde(default)]
    pub ik_disabled: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Default, PartialEq, Debug)]
#[repr(i32)] // Specify the underlying integer type
pub enum InverseKinematicsMode {
    #[default]
    FABRIK,
    Arc,
    Skip,
}
enum_string!(InverseKinematicsMode);

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq, Debug)]
pub struct IkFamily {
    #[serde(default)]
    pub constraint: JointConstraint,
    #[serde(default)]
    pub mode: InverseKinematicsMode,
    #[serde(default)]
    pub target_id: i32,
    #[serde(default)]
    pub bone_ids: Vec<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Armature {
    #[serde(default)]
    pub bones: Vec<Bone>,
    #[serde(default, skip_serializing_if = "are_anims_empty")]
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub styles: Vec<Style>,
    #[serde(skip)]
    pub tex_data: Vec<TextureData>,
}

impl Armature {
    pub fn find_bone_mut(&mut self, id: i32) -> Option<&mut Bone> {
        self.bones.iter_mut().find(|b| b.id == id)
    }

    pub fn set_bone_tex(
        &mut self,
        bone_id: i32,
        new_tex_str: String,
        selected_anim: usize,
        selected_frame: i32,
    ) {
        if selected_anim == usize::MAX {
            let bone_mut = self.bones.iter_mut().find(|b| b.id == bone_id).unwrap();
            bone_mut.tex = new_tex_str;
            let new_tex = self.tex_of(bone_id);
            if new_tex == None {
                return;
            }
            let new_size = new_tex.unwrap().size;

            let bone = self.bones.iter().find(|b| b.id == bone_id).unwrap().clone();
            if !bone.verts_edited {
                let bone_mut = self.bones.iter_mut().find(|b| b.id == bone_id).unwrap();
                (bone_mut.vertices, bone_mut.indices) = renderer::create_tex_rect(&new_size);
            }
        } else {
            let tx = AnimElement::Texture;
            let bone = self.bones.iter().find(|b| b.id == bone_id).unwrap();

            // record texture change in animation
            let anim = &mut self.animations[selected_anim];
            let kf = anim.check_if_in_keyframe(bone_id as i32, selected_frame, tx.clone());
            anim.keyframes[kf].value_str = new_tex_str;

            // add 0th keyframe
            let first = anim.check_if_in_keyframe(bone_id as i32, 0, tx.clone());
            anim.keyframes[first].value_str = bone.tex.clone();
        }
    }

    pub fn new_bone(&mut self, id: i32) -> (Bone, usize) {
        let mut parent_id = -1;
        if self.bones.iter().find(|b| b.id == id) != None {
            parent_id = self.bones.iter().find(|b| b.id == id).unwrap().parent_id;
        }
        let ids = self.bones.iter().map(|a| a.id).collect();

        // set highest zindex so far
        let mut highest_zindex = 0;
        for bone in &self.bones {
            highest_zindex = highest_zindex.max(bone.zindex);
        }

        let new_bone = Bone {
            name: "New Bone".to_string(),
            parent_id,
            id: generate_id(ids),
            scale: Vec2 { x: 1., y: 1. },
            zindex: highest_zindex + 1,
            ik_constraint: JointConstraint::None,
            ik_mode: InverseKinematicsMode::FABRIK,
            ik_target_id: -1,
            ik_family_id: -1,
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

        let kfs = &self.animations[anim_idx].keyframes;

        for b in &mut bones {
            macro_rules! interpolate {
                ($element:expr, $default:expr) => {{
                    self.interpolate_keyframes(anim_idx, b.id, $element, $default, anim_frame)
                }};
            }

            macro_rules! prev_frame {
                ($element:expr, $default:expr) => {{
                    let prev = self.get_prev_frame(anim_frame, kfs, b.id, &$element);
                    if prev != usize::MAX {
                        kfs[prev].value
                    } else {
                        $default
                    }
                }};
            }

            macro_rules! prev_str {
                ($element:expr, $default:expr) => {{
                    let prev = self.get_prev_frame(anim_frame, kfs, b.id, &$element);
                    if prev != usize::MAX {
                        kfs[prev].value_str.clone()
                    } else {
                        $default
                    }
                }};
            }

            // iterable anim interps
            #[rustfmt::skip]
            {
                b.pos.x   = interpolate!(AnimElement::PositionX, b.pos.x  );
                b.pos.y   = interpolate!(AnimElement::PositionY, b.pos.y  );
                b.rot     = interpolate!(AnimElement::Rotation,  b.rot    );
                b.scale.x = interpolate!(AnimElement::ScaleX,    b.scale.x);
                b.scale.y = interpolate!(AnimElement::ScaleY,    b.scale.y);
                b.zindex  = prev_frame!( AnimElement::Zindex,    b.zindex  as f32) as i32;
                b.is_hidden  = prev_frame!( AnimElement::Hidden, bool_as_f32(b.is_hidden)) != 0.;
                b.tex     = prev_str!(   AnimElement::Texture,   b.tex.clone());
            };

            let kfs = &self.animations[anim_idx].keyframes;
            let constraint_frame =
                self.get_prev_frame(anim_frame, kfs, b.id, &AnimElement::IkConstraint);
            if constraint_frame != usize::MAX {
                let constraint = kfs[constraint_frame].value;
                b.ik_constraint = match constraint {
                    1. => JointConstraint::Clockwise,
                    2. => JointConstraint::CounterClockwise,
                    _ => JointConstraint::None,
                };
            }
        }

        bones
    }

    fn get_prev_frame(
        &self,
        frame: i32,
        kfs: &Vec<Keyframe>,
        b_id: i32,
        el: &AnimElement,
    ) -> usize {
        let mut prev = usize::MAX;
        for (i, kf) in kfs.iter().enumerate() {
            if kf.frame <= frame && kf.bone_id == b_id && kf.element == *el {
                prev = i;
            }
        }
        prev
    }

    pub fn interpolate_keyframes(
        &self,
        anim_id: usize,
        bone_id: i32,
        element: AnimElement,
        default: f32,
        frame: i32,
    ) -> f32 {
        let keyframes = &self.animations[anim_id].keyframes;
        let mut prev = self.get_prev_frame(frame, keyframes, bone_id, &element);
        let mut next = usize::MAX;

        for (i, kf) in keyframes.iter().enumerate() {
            if kf.frame > frame && kf.bone_id == bone_id && kf.element == element {
                next = i;
                break;
            }
        }

        // ensure prev and next are pointing somewhere
        if prev == usize::MAX {
            prev = next;
        }
        if next == usize::MAX {
            next = prev;
        }

        if prev == usize::MAX && next == usize::MAX {
            return default;
        }

        let total_frames = keyframes[next].frame - keyframes[prev].frame;
        let current_frame = frame - keyframes[prev].frame;
        self.interpolate(
            current_frame,
            total_frames,
            keyframes[prev].value,
            keyframes[next].value,
        )
    }

    fn interpolate(&self, current: i32, max: i32, start_val: f32, end_val: f32) -> f32 {
        if max == 0 || current >= max {
            return end_val;
        }
        let interp = current as f32 / max as f32;
        let end = end_val - start_val;
        start_val + (end * interp)
    }

    pub fn get_all_parents(&self, bone_id: i32) -> Vec<Bone> {
        let bone = self.bones.iter().find(|b| b.id == bone_id).unwrap().clone();

        // add own bone temporarily
        let mut parents: Vec<Bone> = vec![bone];

        while parents.last().unwrap().parent_id != -1 {
            let id = parents.last().unwrap().parent_id;
            let parent = self.bones.iter().find(|b| b.id == id);
            parents.push(parent.unwrap().clone());
        }

        // remove own bone from list
        parents.remove(0);

        parents
    }

    pub fn offset_pos_by_parent(&mut self, old_parents: Vec<Bone>, bone_id: i32) {
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

    pub fn tex_of(&self, bone_id: i32) -> Option<&Texture> {
        let bone = self.bones.iter().find(|bone| bone.id == bone_id);
        if bone == None {
            return None;
        }
        for style in &self.styles {
            if !style.active {
                continue;
            }
            if let Some(tex) = style.textures.iter().find(|t| t.name == bone.unwrap().tex) {
                return Some(tex);
            }
        }
        None
    }

    pub fn bone_eff(&self, bone_id: i32) -> JointEffector {
        let bone = self.bones.iter().find(|bone| bone.id == bone_id).unwrap();
        let ik_id = bone.ik_family_id;
        if ik_id == -1 {
            return JointEffector::None;
        }

        let family: Vec<&Bone> = self
            .bones
            .iter()
            .filter(|bone| bone.ik_family_id == ik_id)
            .collect();

        let mut count = 0;
        for other_bone in &self.bones {
            if other_bone.ik_family_id != ik_id {
                continue;
            }

            if other_bone.id != bone.id {
                count += 1;
                continue;
            }

            return if count == 0 {
                JointEffector::Start
            } else if count == family.len() - 1 {
                JointEffector::End
            } else {
                JointEffector::Middle
            };
        }

        JointEffector::None
    }

    pub fn is_bone_folded(&self, bone_id: i32) -> bool {
        let mut nb = self.bones.iter().find(|b| b.id == bone_id).unwrap();
        while nb.parent_id != -1 {
            let id = nb.parent_id;
            nb = self.bones.iter().find(|bo| bo.id == id).unwrap();
            if nb.folded {
                return true;
            }
        }

        false
    }

    pub fn tex_data(&self, tex: &Texture) -> Option<&TextureData> {
        self.tex_data.iter().find(|d| d.id == tex.data_id)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct TexAtlas {
    #[serde(skip_deserializing)]
    pub filename: String,
    #[serde(skip_deserializing)]
    pub size: Vec2I,
}

// used for the json
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Root {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub ik_root_ids: Vec<i32>,
    #[serde(default)]
    pub bones: Vec<Bone>,
    #[serde(default, skip_serializing_if = "are_anims_empty")]
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub atlases: Vec<TexAtlas>,
    #[serde(default)]
    pub styles: Vec<Style>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq)]
pub struct Style {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub name: String,
    #[serde(skip)]
    pub active: bool,
    #[serde(default)]
    pub textures: Vec<Texture>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq)]
pub struct Vec2I {
    pub x: i32,
    pub y: i32,
}

impl Vec2I {
    pub const fn new(x: i32, y: i32) -> Vec2I {
        Vec2I { x, y }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq)]
pub struct Texture {
    #[serde(default)]
    pub name: String,

    #[serde(skip)]
    pub offset: Vec2,
    #[serde(skip)]
    pub size: Vec2,

    /// size and offset should be saved as integers
    #[serde(default, rename = "offset")]
    pub ser_offset: Vec2I,
    #[serde(default, rename = "size")]
    pub ser_size: Vec2I,
    #[serde(default)]
    pub atlas_idx: i32,

    #[serde(skip)]
    pub data_id: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq)]
pub struct TextureData {
    #[serde(skip)]
    pub id: i32,
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
    pub fn check_if_in_keyframe(&mut self, id: i32, frame: i32, element: AnimElement) -> usize {
        macro_rules! is_same_frame {
            ($kf:expr) => {
                $kf.frame == frame && $kf.bone_id == id && $kf.element == element
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

    /// runtime: while the editor uses enums for elements, runtimes can use their numerical id
    /// for simplicity and performance
    #[serde(default, rename = "element")]
    pub element_id: i32,
    #[serde(default, rename = "element_str")]
    pub element: AnimElement,

    #[serde(default, skip_serializing_if = "is_str_empty")]
    pub value_str: String,
    #[serde(default, skip_serializing_if = "is_max")]
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
#[rustfmt::skip]
pub enum AnimElement {
    #[default]
    /* 0 */ PositionX,
    /* 1 */ PositionY,
    /* 2 */ Rotation,
    /* 3 */ ScaleX,
    /* 4 */ ScaleY,
    /* 5 */ Zindex,
    /* 6 */ Texture,
    /* 7 */ IkConstraint,
    /* 8 */ Hidden,
}

// iterable anim change icons IDs
#[rustfmt::skip]
pub const ANIM_ICON_ID: [usize; 9] = [
    /* 0 */ 0,
    /* 1 */ 1,
    /* 2 */ 2,
    /* 3 */ 3,
    /* 5 */ 4,
    /* 5 */ 5,
    /* 6 */ 6,
    /* 7 */ 5,
    /* 8 */ 5,
];

#[derive(Default, Clone, PartialEq)]
pub enum ActionType {
    #[default]
    Bone,
    Bones,
    Animation,
    Animations,
    Keyframe,
    Style,
    Styles,
    Texture,
    Textures,
}

#[derive(Default, Clone, PartialEq)]
pub struct Action {
    pub action: ActionType,
    pub bones: Vec<Bone>,
    pub animations: Vec<Animation>,
    pub styles: Vec<Style>,
    pub continued: bool,
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

#[derive(Default)]
pub struct CopyBuffer {
    pub keyframes: Vec<Keyframe>,
    pub anims: Vec<Animation>,
    pub bones: Vec<Bone>,
}

#[derive(Default, Clone, PartialEq)]
pub enum Saving {
    #[default]
    None,
    CustomPath,
    Autosaving,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct StartupResourceItem {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub url_type: StartupItemType,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub items: Vec<StartupResourceItem>,
    #[serde(default)]
    pub update_checker: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default, PartialEq, Clone)]
pub enum StartupItemType {
    #[default]
    Custom,
    DevDocs,
    UserDocs,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct Startup {
    #[serde(default)]
    pub resources: Vec<StartupResourceItem>,
}

#[derive(Default)]
pub struct UndoStates {
    pub undo_actions: Vec<Action>,
    pub redo_actions: Vec<Action>,
    pub prev_undo_actions: Vec<Action>,
    pub temp_actions: Vec<Action>,
}

impl UndoStates {
    pub fn new_undo_bone(&mut self, bone: &Bone) {
        self.undo_actions.push(Action {
            action: ActionType::Bone,
            bones: vec![bone.clone()],
            ..Default::default()
        });
    }

    pub fn new_undo_anim(&mut self, anims: &Animation) {
        self.undo_actions.push(Action {
            action: ActionType::Animation,
            animations: vec![anims.clone()],
            ..Default::default()
        });
    }

    pub fn new_undo_style(&mut self, style: &Style) {
        self.undo_actions.push(Action {
            action: ActionType::Style,
            styles: vec![style.clone()],
            ..Default::default()
        });
    }

    pub fn new_undo_bones(&mut self, bones: &Vec<Bone>) {
        self.undo_actions.push(Action {
            action: ActionType::Bones,
            bones: bones.clone(),
            ..Default::default()
        });
    }

    pub fn new_undo_anims(&mut self, animations: &Vec<Animation>) {
        self.undo_actions.push(Action {
            action: ActionType::Animations,
            animations: animations.clone(),
            ..Default::default()
        });
    }

    pub fn new_undo_styles(&mut self, styles: &Vec<Style>) {
        self.undo_actions.push(Action {
            action: ActionType::Styles,
            styles: styles.clone(),
            ..Default::default()
        });
    }
}

#[derive(Default)]
pub struct Renderer {
    pub window: Vec2,
    pub editing_bone: bool,
    pub dragging_verts: Vec<usize>,
    pub generic_bindgroup: Option<BindGroup>,
    pub ik_arrow_bindgroup: Option<BindGroup>,
    pub changed_vert_id: i32,
    pub changed_vert_init_pos: Option<Vec2>,
    pub initialized_window: bool,
    pub has_loaded: bool,
    pub bone_init_rot: f32,
    pub gridline_gap: i32,
}

impl Renderer {
    pub fn aspect_ratio(&self) -> f32 {
        self.window.y / self.window.x
    }
}

#[derive(Default)]
pub enum Events {
    #[default]
    None,
    CamZoomIn,
    CamZoomOut,
    CamZoomScroll,
    EditModeMove,
    EditModeRotate,
    EditModeScale,
}

#[derive(Default)]
pub struct EventState {
    pub events: Vec<Events>,
    pub values: Vec<f32>,
}

impl EventState {
    pub fn new(&mut self, id: Events) {
        self.events.push(id);
        self.values.push(0.);
    }

    pub fn new_valued(&mut self, id: Events, value: f32) {
        self.events.push(id);
        self.values.push(value);
    }
}

#[derive(Default)]
pub struct Shared {
    pub armature: Armature,
    pub input: InputStates,
    pub cursor_icon: egui::CursorIcon,
    pub ui: Ui,
    pub undo_states: UndoStates,
    pub renderer: Renderer,
    pub events: EventState,
    pub camera: Camera,

    pub recording: bool,
    pub done_recording: bool,
    // mainly used for video, but can also be used for screenshots
    pub rendered_frames: Vec<RenderedFrame>,

    pub edit_mode: EditMode,

    pub recent_file_paths: Vec<String>,

    pub config: Config,

    pub copy_buffer: CopyBuffer,

    pub saving: Arc<Mutex<Saving>>,
    pub save_finished: Arc<Mutex<bool>>,

    pub time: f32,

    pub last_autosave: f32,

    pub screenshot_res: Vec2,

    pub file_name: Arc<Mutex<String>>,
    pub img_contents: Arc<Mutex<Vec<u8>>>,
    pub import_contents: Arc<Mutex<Vec<u8>>>,
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

    pub fn selected_bone_id(&self) -> i32 {
        if let Some(bone) = self.selected_bone() {
            return bone.id;
        }

        -1
    }

    pub fn selected_bone_mut(&mut self) -> Option<&mut Bone> {
        if self.ui.selected_bone_idx != usize::MAX {
            return Some(&mut self.armature.bones[self.ui.selected_bone_idx]);
        }
        None
    }

    pub fn save_edited_bone(&mut self) {
        if self.ui.is_animating() {
            let anim = self.selected_animation().unwrap().clone();
            self.undo_states.new_undo_anim(&anim);
        } else {
            self.undo_states.new_undo_anims(&self.armature.animations);
        }
    }

    pub fn selected_set(&self) -> Option<&Style> {
        self.armature
            .styles
            .iter()
            .find(|set| set.id == self.ui.selected_style)
    }

    pub fn selected_set_mut(&mut self) -> Option<&mut Style> {
        self.armature
            .styles
            .iter_mut()
            .find(|set| set.id == self.ui.selected_style)
    }

    pub fn open_style_modal(&mut self) {
        self.ui.styles_modal = true;
    }

    pub fn animate_bones(&mut self) -> Vec<Bone> {
        // runtime:
        // armature bones should normally be mutable to animation for smoothing,
        // but that's not ideal when editing
        let mut animated_bones = self.armature.bones.clone();

        let anims = &self.armature.animations;
        let is_any_anim_playing = anims.iter().find(|anim| anim.elapsed != None) != None;

        let anim = &self.ui.anim;
        if is_any_anim_playing {
            // runtime: playing animations (single & simultaneous)
            for a in 0..self.armature.animations.len() {
                let anim = &mut self.armature.animations[a];
                if anim.elapsed == None {
                    continue;
                }
                let frame = anim.set_frame();
                animated_bones = self.armature.animate(a, frame, Some(&animated_bones));
            }
        } else if anim.open && anim.selected != usize::MAX && anim.selected_frame != -1 {
            let frame = anim.selected_frame;

            // display the selected animation's frame
            animated_bones = self.armature.animate(anim.selected, frame, None);
        }

        animated_bones
    }

    pub fn world_camera(&self) -> Camera {
        let mut cam = self.camera.clone();
        match self.config.layout {
            UiLayout::Right => cam.pos.x += 1500. * self.renderer.aspect_ratio(),
            UiLayout::Left => cam.pos.x -= 1500. * self.renderer.aspect_ratio(),
            _ => {}
        };
        cam
    }

    pub fn edit_bone(
        &mut self,
        bone_id: i32,
        element: &AnimElement,
        value: f32,
        anim_id: usize,
        anim_frame: i32,
    ) {
        self.save_edited_bone();
        let bones = &mut self.armature.bones;
        let bone = bones.iter_mut().find(|b| b.id == bone_id).unwrap();
        let mut init_value = 0.;

        // do nothing if anim is playing and edit_while_playing config is false
        let anims = &self.armature.animations;
        let is_any_anim_playing = anims.iter().find(|anim| anim.elapsed != None) != None;
        if !self.config.edit_while_playing && is_any_anim_playing {
            return;
        }

        macro_rules! set {
            ($field:expr) => {{
                init_value = $field;
                if anim_id == usize::MAX {
                    $field = value;
                }
            }};
        }

        match element {
            AnimElement::PositionX => set!(bone.pos.x),
            AnimElement::PositionY => set!(bone.pos.y),
            AnimElement::Rotation => set!(bone.rot),
            AnimElement::ScaleX => set!(bone.scale.x),
            AnimElement::ScaleY => set!(bone.scale.y),
            AnimElement::Zindex => {
                init_value = bone.zindex as f32;
                if anim_id == usize::MAX {
                    bone.zindex = value as i32
                }
            }
            AnimElement::Texture => { /* handled in set_bone_tex() */ }
            AnimElement::IkConstraint => {
                init_value = (bone.ik_constraint as usize) as f32;
                if anim_id == usize::MAX {
                    bone.ik_constraint = match value {
                        1. => JointConstraint::Clockwise,
                        2. => JointConstraint::CounterClockwise,
                        _ => JointConstraint::None,
                    }
                }
            }
            AnimElement::Hidden => {
                init_value = bool_as_f32(bone.is_hidden);
                if anim_id == usize::MAX {
                    bone.is_hidden = f32_as_bool(value)
                }
            }
        };

        if anim_id == usize::MAX {
            return;
        }

        macro_rules! check_kf {
            ($kf:expr) => {
                $kf.frame == 0 && $kf.element == *element && $kf.bone_id == bone_id
            };
        }

        let anim = &mut self.armature.animations;

        let has_0th = anim[anim_id].keyframes.iter().find(|kf| check_kf!(kf)) != None;
        if anim_frame != 0 && !has_0th {
            anim[anim_id].check_if_in_keyframe(bone_id, 0, element.clone());
            let oth_frame = anim[anim_id].keyframes.iter_mut().find(|kf| check_kf!(kf));
            oth_frame.unwrap().value = init_value;
        }
        let frame = anim[anim_id].check_if_in_keyframe(bone_id, anim_frame, element.clone());
        anim[anim_id].keyframes[frame].value = value;
    }

    pub fn sel_tex_img(&self) -> image::DynamicImage {
        let sel_id = self.selected_bone().unwrap().id;
        let tex = self.armature.tex_of(sel_id).unwrap();
        self.armature.tex_data(tex).unwrap().image.clone()
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
    500
}

fn is_neg_one(value: &i32) -> bool {
    *value == -1
}

fn is_max(value: &f32) -> bool {
    *value == f32::MAX
}

fn are_verts_empty(value: &Vec<Vertex>) -> bool {
    *value == vec![]
}

fn are_indices_empty<T: std::cmp::PartialEq<Vec<u32>>>(value: &T) -> bool {
    *value == vec![]
}

fn are_weights_empty<T: std::cmp::PartialEq<Vec<BoneBind>>>(value: &T) -> bool {
    *value == vec![]
}

fn are_anims_empty(value: &Vec<Animation>) -> bool {
    *value == vec![]
}

fn is_i32_empty(value: &Vec<i32>) -> bool {
    value.len() == 0
}

fn is_str_empty(value: &String) -> bool {
    value == ""
}

fn no_constraints(value: &JointConstraint) -> bool {
    *value == JointConstraint::Skip
}

fn no_ik_mode(value: &InverseKinematicsMode) -> bool {
    *value == InverseKinematicsMode::Skip
}

fn is_false(value: &bool) -> bool {
    *value == false
}

fn f32_as_bool(value: f32) -> bool {
    value == 1.
}

fn bool_as_f32(value: bool) -> f32 {
    if value {
        1.
    } else {
        0.
    }
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
