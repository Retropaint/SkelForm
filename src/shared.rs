//! Easily-accessible and frequently-shared data.

use crate::*;

use std::{
    fmt,
    ops::{DivAssign, MulAssign},
    path::PathBuf,
    str::FromStr,
};

use std::sync::Mutex;
use wgpu::BindGroup;

use strum::{EnumString, FromRepr};

use serde::{Deserialize, Serialize};

#[rustfmt::skip]
#[repr(C)]
#[derive(Debug,Serialize,Deserialize,Default,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable)]
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
#[serde(default)]
pub struct Vertex {
    pub id: u32,
    pub pos: Vec2,
    pub uv: Vec2,
    pub init_pos: Vec2,
    #[serde(skip)]
    pub color: VertexColor,
    #[serde(skip)]
    pub add_color: VertexColor,
    #[serde(skip)]
    pub tint: TintColor,
    #[serde(skip)]
    pub offset_rot: f32,
}

// The vertex data supplied to wgpu.
#[rustfmt::skip]
#[repr(C)]
#[derive(PartialEq,serde::Serialize,serde::Deserialize,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable,Debug,Default)]
#[serde(default)]
pub struct GpuVertex {
    pub pos: Vec2,
    pub uv: Vec2,
    #[serde(skip)]
    pub color: VertexColor,
    #[serde(skip)]
    pub add_color: VertexColor,
    #[serde(skip)]
    pub tint: TintColor,
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
            tint: TintColor::new(1., 1., 1., 1.),
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
            tint: vert.tint,
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

#[rustfmt::skip]
#[repr(C)]
#[derive(PartialEq, Copy, Clone, serde::Deserialize, serde::Serialize, Default, Debug, bytemuck::Pod,bytemuck::Zeroable)]
pub struct TintColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl TintColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> TintColor {
        TintColor { r, g, b, a }
    }
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
    #[serde(skip)]
    pub on_ui: bool,
    #[serde(skip)]
    pub window: Vec2,
}

impl Camera {
    pub fn aspect_ratio(&self) -> f32 {
        self.window.y / self.window.x
    }
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

#[derive(Clone, Default, PartialEq)]
pub enum ExportState {
    #[default]
    Armature,
    Spritesheet,
}

#[derive(Clone, Default, PartialEq, Debug, FromRepr)]
pub enum PolarId {
    #[default]
    DeleteBone,
    Exiting,
    DeleteAnim,
    DeleteFile,
    DeleteTex,
    DeleteStyle,
    DeleteKeyframeLine,
    NewUpdate,
    OpenCrashlog,
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
        self.hide = true;
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

#[derive(Clone, Default, PartialEq)]
pub enum Warnings {
    #[default]
    SameZIndex,
    NoIkTarget,
    OnlyIk,
    UnboundBind,
    NoVertsInBind,
    OnlyPath,
    NoWeights,
    BoneOutOfFamily,
    EmptyStyles,
    UnusedTextures,
}

#[derive(Clone, Default, PartialEq)]
pub struct Warning {
    pub warn_type: Warnings,
    pub ids: Vec<usize>,
    pub value: f32,
    pub str_values: Vec<String>,
}

impl Warning {
    pub fn new(warn_type: Warnings, ids: Vec<usize>) -> Self {
        Self {
            warn_type,
            ids,
            value: 0.,
            str_values: vec![],
        }
    }

    pub fn valued(warn_type: Warnings, ids: Vec<usize>, value: f32) -> Self {
        Self {
            warn_type,
            ids,
            value,
            str_values: vec![],
        }
    }

    pub fn full(warn_type: Warnings, ids: Vec<usize>, value: f32, str_values: Vec<String>) -> Self {
        Self {
            warn_type,
            ids,
            value,
            str_values,
        }
    }
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

    pub rename_id: String,
    // the initial value of what is being edited via input
    pub edit_value: Option<String>,
    pub last_rename_id: String,
    pub last_edit_value: Option<String>,

    // id to identify actions for polar (yes-no) dialog
    pub polar_id: PolarId,

    pub headline: String,

    pub states: Vec<UiState>,

    pub scale: f32,

    /// Ensures that auto-focused behaviour only runs once
    pub input_focused: bool,

    // context menu stuff

    // determines if context menu should close on next click
    pub context_menu: ContextMenu,

    pub settings_state: SettingsState,
    pub translucent_settings: bool,

    pub changing_key: String,

    pub hovering_tex: i32,
    pub hovering_bone: i32,
    pub hovering_set: i32,
    pub hovering_anim: i32,
    pub hovering_style_bone: i32,
    pub hovering_setting: Option<shared::SettingsState>,

    pub showing_samples: bool,

    pub selected_path: String,

    // not visually indicated; just used for `double click > rename` logic
    pub selected_tex: i32,

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
    pub crashed_last_time: bool,
    pub never_donate: bool,
    pub atlas_image: Option<Vec2>,
    pub dragging_slice: usize,
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
    pub focus_style_dropdown: bool,
    pub donating_modal: bool,
    pub atlas_modal: bool,
    pub export_modal: bool,
    pub checking_update: bool,
    pub update_request_sent: bool,
    pub new_version: String,

    loc_strings: std::collections::HashMap<String, String>,
    pub cursor_icon: egui::CursorIcon,
    pub last_pressed: Option<egui::Key>,
    pub recent_file_paths: Vec<String>,

    pub file_path: Arc<Mutex<Vec<PathBuf>>>,
    pub file_type: Arc<Mutex<i32>>,

    pub saving: Arc<Mutex<Saving>>,
    pub save_finished: Arc<Mutex<bool>>,
    pub export_finished: Arc<Mutex<bool>>,
    pub can_quit: bool,
    pub warnings: Vec<Warning>,
    pub warnings_open: bool,
    pub save_path: Option<PathBuf>,
    pub changed_window_name: bool,

    // export options
    pub sprite_size: Vec2,
    pub sprites_per_row: i32,
    pub spritesheet_elapsed: Option<Instant>,
    pub rendered_spritesheets: Vec<Vec<RenderedFrame>>,
    pub exporting_anims: Vec<bool>,
    pub image_sequences: bool,
    pub exporting_video_type: ExportVideoType,
    pub exporting_video_anim: usize,
    pub exporting_video_encoder: ExportVideoEncoder,
    pub open_after_export: bool,
    pub use_system_ffmpeg: bool,
    pub video_clear_bg: Color,
    pub anim_cycles: i32,
    pub mapped_frames: Arc<Mutex<usize>>,
    pub custom_error: String,

    // used for the UpdateConfig event
    pub updated_config: Config,

    pub bone_tops: BoneTops,

    pub dragging_handles: bool,

    pub tracing: bool,
    pub tracing_gap: f32,
    pub tracing_padding: f32,

    pub pointer_on_timeline: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Eq, Debug, Clone)]
pub enum ExportVideoType {
    #[default]
    None,
    Mp4,
    Gif,
}
enum_string!(ExportVideoType);

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Eq, Debug, Clone)]
pub enum ExportVideoEncoder {
    #[default]
    Libx264,
    AV1,
}
enum_string!(ExportVideoEncoder);

impl Ui {
    pub fn is_animating(&self, edit_mode: &EditMode, selections: &SelectionState) -> bool {
        edit_mode.anim_open && selections.anim != usize::MAX
    }

    pub fn context_id_parsed(&self) -> i32 {
        let raw_id = self.context_menu.id.split('_').collect::<Vec<_>>();
        raw_id[1].parse::<i32>().unwrap()
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

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct Config {
    #[serde(default = "default_one")]
    pub ui_scale: f32,
    #[serde(default = "gridline_default")]
    pub gridline_gap: i32,
    pub skip_startup: bool,
    pub autosave_frequency: i32,
    pub exact_bone_select: bool,
    pub gridline_front: bool,
    pub keep_tex_str: bool,
    pub edit_while_playing: bool,
    pub layout: UiLayout,
    pub ignore_donate: bool,
    pub pixel_magnification: i32,
    pub keys: KeyboardConfig,
    pub propagate_visibility: bool,

    #[serde(skip)]
    pub colors: ColorConfig,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
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
    pub warning_text: Color,
    pub inverse_kinematics: Color,
    pub meshdef: Color,
    pub texture: Color,
    pub ik_target: Color,
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
            pixel_magnification: 1,
            propagate_visibility: false,
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
            warning_text: Color::new(214, 168, 0, 0),
            meshdef: Color::new(0, 125, 20, 255),
            texture: Color::new(200, 200, 200, 255),
            inverse_kinematics: Color::new(188, 188, 0, 255),
            ik_target: Color::new(90, 90, 150, 255),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct KeyboardConfig {
    pub next_anim_frame: egui::KeyboardShortcut,
    pub prev_anim_frame: egui::KeyboardShortcut,
    pub zoom_in_camera: egui::KeyboardShortcut,
    pub zoom_out_camera: egui::KeyboardShortcut,
    pub undo: egui::KeyboardShortcut,
    pub redo: egui::KeyboardShortcut,
    pub save: egui::KeyboardShortcut,
    pub save_as: egui::KeyboardShortcut,
    pub export: egui::KeyboardShortcut,
    pub open: egui::KeyboardShortcut,
    pub cancel: egui::KeyboardShortcut,
    pub copy: egui::KeyboardShortcut,
    pub paste: egui::KeyboardShortcut,
    pub timeline_zoom_mode: egui::KeyboardShortcut,
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
            next_anim_frame:    regular_key!(egui::Key::ArrowRight),
            prev_anim_frame:    regular_key!(egui::Key::ArrowLeft),
            zoom_in_camera:     regular_key!(egui::Key::Equals),
            zoom_out_camera:    regular_key!(egui::Key::Minus),
            cancel:             regular_key!(egui::Key::Escape),
            undo:               shortcut_key!(egui::Modifiers::COMMAND, egui::Key::Z),
            redo:               shortcut_key!(egui::Modifiers::COMMAND, egui::Key::Y),
            save:               shortcut_key!(egui::Modifiers::COMMAND, egui::Key::S),
            save_as:            shortcut_key!(egui::Modifiers::SHIFT, egui::Key::S),
            export:             shortcut_key!(egui::Modifiers::COMMAND, egui::Key::E),
            open:               shortcut_key!(egui::Modifiers::COMMAND, egui::Key::O),
            copy:               shortcut_key!(egui::Modifiers::COMMAND, egui::Key::C),
            paste:              shortcut_key!(egui::Modifiers::COMMAND, egui::Key::V),
            timeline_zoom_mode: shortcut_key!(egui::Modifiers::COMMAND, egui::Key::F30),
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
    pub hovering_frame: i32,
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
    pub deleting_line_bone_id: i32,
    pub deleting_line_element: AnimElement,
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

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Default, PartialEq, Debug, EnumString,
)]
pub enum JointConstraint {
    #[default]
    None,
    Clockwise,
    CounterClockwise,
    Skip,
}

enum_string!(JointConstraint);

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default, Debug)]
#[serde(default)]
pub struct Bone {
    pub id: i32,
    pub name: String,
    pub parent_id: i32,
    #[serde(default, skip_serializing_if = "is_str_empty")]
    pub tex: String,
    #[serde(default = "default_tint", skip_serializing_if = "is_tint_white")]
    pub tint: TintColor,
    #[serde(default, skip_serializing_if = "is_neg_one")]
    pub zindex: i32,
    pub pos: Vec2,
    pub scale: Vec2,
    pub rot: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,

    #[serde(default = "default_neg_one")]
    pub ik_family_id: i32,
    #[rustfmt::skip]
    #[serde(default, skip_serializing_if = "no_constraints")]
    pub ik_constraint: JointConstraint,
    #[serde(default, skip_serializing_if = "no_ik_mode")]
    pub ik_mode: InverseKinematicsMode,
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
    #[serde(default, skip_serializing_if = "no_constraints", skip_deserializing)]
    pub init_ik_constraint: JointConstraint,
    #[serde(default, skip_serializing_if = "no_ik_mode", skip_deserializing)]
    pub init_ik_mode: InverseKinematicsMode,
    #[serde(default, skip_serializing_if = "is_false", skip_deserializing)]
    pub init_hidden: bool,
    #[serde(default, skip_serializing_if = "is_str_empty", skip_deserializing)]
    pub init_tex: String,
    #[serde(default = "default_tint", skip_serializing_if = "is_tint_white")]
    pub init_tint: TintColor,
    #[serde(default, skip_serializing_if = "is_neg_one")]
    pub init_zindex: i32,

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
    pub effects_folded: bool,
    #[serde(skip)]
    pub world_verts: Vec<Vertex>,
    #[serde(skip)]
    pub ik_disabled: bool,
    #[serde(skip)]
    pub locked: bool,
    #[serde(skip)]
    pub vertex_buffer: Option<wgpu::Buffer>,
    #[serde(skip)]
    pub index_buffer: Option<wgpu::Buffer>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default, Debug)]
#[serde(default)]
pub struct BoneBind {
    #[serde(default = "default_neg_one")]
    pub bone_id: i32,
    pub is_path: bool,
    pub verts: Vec<BoneBindVert>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Default, Debug)]
#[serde(default)]
pub struct BoneBindVert {
    pub id: i32,
    pub weight: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct EditorStyle {
    pub active: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct EditorOptions {
    pub camera: Camera,
    pub bones: Vec<EditorBone>,
    pub styles: Vec<EditorStyle>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct EditorBone {
    pub folded: bool,
    pub ik_folded: bool,
    pub meshdef_folded: bool,
    pub effects_folded: bool,
    pub ik_disabled: bool,
    pub locked: bool,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Copy,
    Clone,
    Default,
    PartialEq,
    Debug,
    EnumString,
    FromRepr,
)]
#[repr(i32)]
pub enum InverseKinematicsMode {
    #[default]
    FABRIK,
    Arc,
    Skip,
}
enum_string!(InverseKinematicsMode);

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq, Debug)]
#[serde(default)]
pub struct IkFamily {
    pub constraint: JointConstraint,
    pub mode: InverseKinematicsMode,
    pub target_id: i32,
    pub bone_ids: Vec<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct Armature {
    pub bones: Vec<Bone>,
    #[serde(default, skip_serializing_if = "are_anims_empty")]
    pub animations: Vec<Animation>,
    pub styles: Vec<Style>,
    #[serde(skip)]
    pub tex_data: Vec<TextureData>,
    #[serde(skip)]
    pub animated_bones: Vec<Bone>,
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

            // add 0th keyframe if that wasn't the selected frame
            if selected_frame != 0 {
                let first = anim.check_if_in_keyframe(bone_id as i32, 0, tx.clone());
                if first == usize::MAX {
                    anim.keyframes[first].value_str = bone.tex.clone();
                }
            }
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
            tint: default_tint(),
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
                    let prev = utils::get_prev_frame(anim_frame, kfs, b.id, &$element);
                    if prev != usize::MAX {
                        kfs[prev].value
                    } else {
                        $default
                    }
                }};
            }

            macro_rules! prev_str {
                ($element:expr, $default:expr) => {{
                    let prev = utils::get_prev_frame(anim_frame, kfs, b.id, &$element);
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
                b.tint.r =  interpolate!(AnimElement::TintR,     b.tint.r);
                b.tint.g =  interpolate!(AnimElement::TintG,     b.tint.g);
                b.tint.b =  interpolate!(AnimElement::TintB,     b.tint.b);
                b.tint.a =  interpolate!(AnimElement::TintA,     b.tint.a);
                b.zindex  = prev_frame!( AnimElement::Zindex,    b.zindex  as f32) as i32;
                b.hidden  = prev_frame!( AnimElement::Hidden,    bool_as_f32(b.hidden)) != 0.;
                b.tex     = prev_str!(   AnimElement::Texture,   b.tex.clone());
            };

            macro_rules! prev_frame {
                ($field:expr, $anim_element:expr, $enum:ident) => {
                    let kfs = &self.animations[anim_idx].keyframes;
                    let prev_frame = utils::get_prev_frame(anim_frame, kfs, b.id, &$anim_element);
                    if prev_frame != usize::MAX {
                        $field = $enum::from_str(&kfs[prev_frame].value_str).unwrap();
                    }
                };
            }

            prev_frame!(b.ik_constraint, AnimElement::IkConstraint, JointConstraint);
            prev_frame!(b.ik_mode, AnimElement::IkMode, InverseKinematicsMode);
        }

        bones
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
        let mut prev = utils::get_prev_frame(frame, keyframes, bone_id, &element);
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

        utils::interp(
            frame - keyframes[prev].frame,
            keyframes[next].frame - keyframes[prev].frame,
            keyframes[prev].value,
            keyframes[next].value,
            keyframes[next].start_handle,
            keyframes[next].end_handle,
        )
    }

    pub fn get_all_parents(&self, is_anim: bool, bone_id: i32) -> Vec<Bone> {
        let bones = if is_anim {
            &self.animated_bones
        } else {
            &self.bones
        };

        let bone = bones.iter().find(|b| b.id == bone_id).unwrap().clone();

        // add own bone temporarily
        let mut parents: Vec<Bone> = vec![bone];

        while parents.last().unwrap().parent_id != -1 {
            let id = parents.last().unwrap().parent_id;
            let parent = bones.iter().find(|b| b.id == id);
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

        let new_parents = self.get_all_parents(false, bone_id);
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

    pub fn anim_tex_of(&self, bone_id: i32) -> Option<&Texture> {
        let bone = self.animated_bones.iter().find(|bone| bone.id == bone_id);
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

    pub fn sel_style(&self, selection: &SelectionState) -> Option<&Style> {
        self.styles.iter().find(|s| s.id == selection.style)
    }

    pub fn sel_style_mut(&mut self, selection: &SelectionState) -> Option<&mut Style> {
        self.styles.iter_mut().find(|s| s.id == selection.style)
    }

    pub fn sel_anim(&self, selection: &SelectionState) -> Option<&Animation> {
        if selection.anim > self.animations.len() {
            return None;
        }
        Some(&self.animations[selection.anim as usize])
    }

    pub fn sel_anim_mut(&mut self, selections: &SelectionState) -> Option<&mut Animation> {
        if selections.anim > self.animations.len() {
            return None;
        }
        Some(&mut self.animations[selections.anim as usize])
    }

    pub fn sel_bone(&self, selections: &SelectionState) -> Option<&Bone> {
        if selections.bone_idx != usize::MAX && selections.bone_idx < self.bones.len() {
            return Some(&self.bones[selections.bone_idx]);
        }
        None
    }

    pub fn sel_bone_mut(&mut self, selections: &SelectionState) -> Option<&mut Bone> {
        if selections.bone_idx != usize::MAX && selections.bone_idx < self.bones.len() {
            return Some(&mut self.bones[selections.bone_idx]);
        }
        None
    }

    pub fn is_bone_hidden(&self, is_anim: bool, propagate: bool, bone_id: i32) -> bool {
        let bones = if is_anim {
            &self.animated_bones
        } else {
            &self.bones
        };

        let bone = bones.iter().find(|b| b.id == bone_id);

        if bone == None {
            return false;
        }
        if !propagate {
            return bone.unwrap().hidden;
        }
        if bone.unwrap().hidden {
            return true;
        }

        let parents = self.get_all_parents(is_anim, bone_id);
        for parent in &parents {
            if parent.hidden {
                return true;
            }
        }

        false
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct TexAtlas {
    pub filename: String,
    #[serde(skip_deserializing)]
    pub size: Vec2I,
}

// used for the json
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
#[serde(default)]
pub struct Root {
    pub version: String,
    pub ik_root_ids: Vec<i32>,
    pub baked_ik: bool,
    pub img_format: ExportImgFormat,
    #[serde(default, skip_serializing_if = "is_color_empty")]
    pub clear_color: Color,
    pub bones: Vec<Bone>,
    #[serde(default, skip_serializing_if = "are_anims_empty")]
    pub animations: Vec<Animation>,
    pub atlases: Vec<TexAtlas>,
    pub styles: Vec<Style>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct Style {
    pub id: i32,
    pub name: String,
    #[serde(skip)]
    pub active: bool,
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
#[serde(default)]
pub struct Texture {
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
#[serde(default)]
pub struct Animation {
    pub name: String,
    pub id: i32,
    pub fps: i32,
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
            start_handle: utils::interp_preset(HandlePreset::Linear).0,
            end_handle: utils::interp_preset(HandlePreset::Linear).1,
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
#[serde(default)]
pub struct Keyframe {
    pub frame: i32,
    pub bone_id: i32,

    #[serde(default)]
    pub element: AnimElement,

    #[serde(default, skip_serializing_if = "is_str_empty")]
    pub value_str: String,
    #[serde(default, skip_serializing_if = "is_max")]
    pub value: f32,

    #[serde(default)]
    pub start_handle: Vec2,
    #[serde(default)]
    pub end_handle: Vec2,
    // unused in editor and official runtimes - just a helper for personal runtimes
    // to hardcode interpolations instead of implementing beziers
    #[serde(default)]
    pub handle_preset: HandlePreset,

    #[serde(skip)]
    pub label_top: f32,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default, Debug, FromRepr)]
pub enum HandlePreset {
    #[default]
    Linear,
    SineIn,
    SineOut,
    SineInOut,
    None,
    Custom,
}
enum_string!(HandlePreset);

#[derive(
    Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, Clone, Default, Debug, FromRepr
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
    /* 9 */ IkMode,
    /* 10 */ IkFamilyId,
    /* 11 */ TintR,
    /* 12 */ TintG,
    /* 13 */ TintB,
    /* 14 */ TintA,
    /* 15 */ Locked,
}

// iterable anim change icons IDs
#[rustfmt::skip]
pub const ANIM_ICON_ID: [usize; 15] = [
    /* 0 */ 0,
    /* 1 */ 1,
    /* 2 */ 2,
    /* 3 */ 3,
    /* 5 */ 4,
    /* 5 */ 5,
    /* 6 */ 6,
    /* 7 */ 5,
    /* 8 */ 5,
    /* 9 */ 7,
    /* 10 */ 5,
    /* 11 */ 8,
    /* 12 */ 9,
    /* 13 */ 10,
    /* 14 */ 11,
];

#[derive(Default, Clone, PartialEq, Debug)]
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
enum_string!(ActionType);

#[derive(Default, Clone, PartialEq)]
pub struct Action {
    pub action: ActionType,
    pub bones: Vec<Bone>,
    pub animations: Vec<Animation>,
    pub styles: Vec<Style>,
    pub continued: bool,
}
enum_string!(AnimElement);

#[derive(Default, Debug, Clone)]
pub struct BoneTops {
    pub tops: Vec<BoneTop>,
}

#[derive(Default, PartialEq, Clone)]
pub enum EditModes {
    #[default]
    Move,
    Rotate,
    Scale,
}

#[derive(Default, PartialEq, Clone, FromRepr, serde::Serialize, serde::Deserialize, Debug)]
pub enum ExportImgFormat {
    #[default]
    PNG,
    JPG,
}
enum_string!(ExportImgFormat);

#[derive(Default, Clone)]
pub struct EditMode {
    pub current: EditModes,
    pub is_moving: bool,
    pub is_scaling: bool,
    pub is_rotating: bool,
    pub showing_mesh: bool,
    pub setting_bind_verts: bool,
    pub setting_bind_bone: bool,
    pub setting_ik_target: bool,
    pub anim_open: bool,
    pub time: f32,
    pub export_bake_ik: bool,
    pub export_exclude_ik: bool,
    pub export_img_format: ExportImgFormat,
    pub export_clear_color: Color,
    pub onion_layers: bool,
}

#[derive(Default, PartialEq, Debug, Clone)]
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

#[derive(Default, Clone, PartialEq, Debug)]
pub enum Saving {
    #[default]
    None,
    CustomPath,
    Autosaving,
    Exporting,
    Spritesheet,
    Video,
}
enum_string!(Saving);

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
#[serde(default)]
pub struct StartupResourceItem {
    pub code: String,
    pub url_type: StartupItemType,
    pub url: String,
    pub items: Vec<StartupResourceItem>,
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
#[serde(default)]
pub struct Startup {
    pub resources: Vec<StartupResourceItem>,
}

#[derive(Default)]
pub struct UndoStates {
    pub undo_actions: Vec<Action>,
    pub redo_actions: Vec<Action>,
    pub unsaved_undo_actions: usize,
    pub prev_undo_actions: usize,
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
    pub editing_bone: bool,
    pub dragging_verts: Vec<usize>,
    pub generic_bindgroup: Option<BindGroup>,
    pub changed_vert_id: i32,
    pub changed_vert_init_pos: Option<Vec2>,
    pub initialized_window: bool,
    pub has_loaded: bool,
    pub bone_init_rot: f32,
    pub new_vert: Option<Vertex>,
    pub started_dragging_verts: bool,
    pub sel_temp_bone: Option<Bone>,
    pub temp_bones: Vec<Bone>,
    pub vertex_buffer: Option<wgpu::Buffer>,
    pub index_buffer: Option<wgpu::Buffer>,
    pub bone_vertex_buffer: Option<wgpu::Buffer>,
    pub bone_index_buffer: Option<wgpu::Buffer>,
    pub prev_onion_vertex_buffer: Option<wgpu::Buffer>,
    pub prev_onion_index_buffer: Option<wgpu::Buffer>,
    pub next_onion_vertex_buffer: Option<wgpu::Buffer>,
    pub next_onion_index_buffer: Option<wgpu::Buffer>,
}

#[derive(Default, PartialEq, Clone, Debug)]
pub enum Events {
    #[default]
    None,
    Undo,
    Redo,

    CamZoomIn,
    CamZoomOut,
    CamZoomScroll,

    EditModeMove,
    EditModeRotate,
    EditModeScale,

    SelectBone,
    SelectAnimFrame,
    SelectAnim,
    SelectStyle,

    DeleteBone,
    DeleteAnim,
    DeleteTex,
    DeleteStyle,
    DeleteKeyframe,
    RemoveVertex,
    RemoveTriangle,
    RemoveKeyframesByFrame,
    DeleteKeyframeLine,

    CopyBone,
    PasteBone,
    CopyKeyframe,
    CopyKeyframesInFrame,
    PasteKeyframes,

    NewAnimation,
    NewStyle,
    NewBone,
    NewArmature,
    NewVertex,

    RenameAnim,
    RenameStyle,
    RenameTex,
    RenameBone,

    SetKeyframeFrame,
    SetAllKeyframesFrame,
    SetBoneTexture,

    DragBone,
    DragVertex,
    MigrateTexture,
    MoveTexture,
    MoveStyle,

    ToggleAnimPlaying,
    ToggleStyleActive,
    ToggleShowingMesh,
    ToggleBindingVerts,
    ToggleSettingIkTarget,
    ToggleAnimPanelOpen,
    ToggleIkFolded,
    ToggleIkDisabled,
    ToggleMeshdefFolded,
    ToggleEffectsFolded,
    ToggleBindPathing,
    ToggleBakingIk,
    ToggleExcludeIk,

    OpenModal,
    UnselectAll,
    OpenPolarModal,
    PointerOnUi,

    DuplicateAnim,
    ToggleBoneFolded,
    EditBone,
    SaveEditedBone,
    SaveBone,
    SaveAnimation,
    ApplySettings,
    ResetConfig,
    EditCamera,
    ClickVertex,
    AdjustVertex,
    CancelPendingTexture,
    AdjustKeyframesByFPS,
    ResetVertices,
    SelectBind,
    RemoveIkTarget,
    CenterBoneVerts,
    TraceBoneVerts,
    SetBindWeight,
    OpenFileErrModal,
    SetExportClearColor,
    SetExportImgFormat,
    OpenExportModal,
    UpdateConfig,
    UpdateKeyframeTransition,
    ToggleOnionLayers,
}

enum_string!(Events);

#[derive(Default)]
pub struct EventState {
    pub events: Vec<Events>,
    pub values: Vec<f32>,
    pub str_values: Vec<String>,
}

macro_rules! generic_event {
    ($name:ident, $enum:expr) => {
        pub fn $name(&mut self) {
            self.events.push($enum);
            self.values.push(-1.);
            self.str_values.push("".to_string());
        }
    };
}

macro_rules! event_with_value {
    ($name:ident, $enum:expr, $id_name:ident, $id_type:ident) => {
        pub fn $name(&mut self, $id_name: $id_type) {
            self.events.push($enum);
            self.values.push($id_name as f32);
            self.str_values.push("".to_string());
        }
    };
}

impl EventState {
    generic_event!(new_animation, Events::NewAnimation);
    generic_event!(apply_settings, Events::ApplySettings);
    generic_event!(reset_config, Events::ResetConfig);
    generic_event!(new_bone, Events::NewBone);
    generic_event!(new_style, Events::NewStyle);
    generic_event!(unselect_all, Events::UnselectAll);
    generic_event!(cam_zoom_scroll, Events::CamZoomScroll);
    generic_event!(undo, Events::Undo);
    generic_event!(redo, Events::Redo);
    generic_event!(cam_zoom_in, Events::CamZoomIn);
    generic_event!(cam_zoom_out, Events::CamZoomOut);
    generic_event!(edit_mode_move, Events::EditModeMove);
    generic_event!(edit_mode_rotate, Events::EditModeRotate);
    generic_event!(edit_mode_scale, Events::EditModeScale);
    generic_event!(new_armature, Events::NewArmature);
    generic_event!(new_vertex, Events::NewVertex);
    generic_event!(cancel_pending_texture, Events::CancelPendingTexture);
    generic_event!(paste_keyframes, Events::PasteKeyframes);
    generic_event!(reset_vertices, Events::ResetVertices);
    generic_event!(remove_ik_target, Events::RemoveIkTarget);
    generic_event!(center_bone_verts, Events::CenterBoneVerts);
    generic_event!(trace_bone_verts, Events::TraceBoneVerts);
    generic_event!(open_export_modal, Events::OpenExportModal);
    generic_event!(update_config, Events::UpdateConfig);
    generic_event!(copy_keyframes_in_frame, Events::CopyKeyframesInFrame);
    generic_event!(save_animation, Events::SaveAnimation);
    event_with_value!(select_anim, Events::SelectAnim, anim_id, usize);
    event_with_value!(select_style, Events::SelectStyle, style_id, usize);
    event_with_value!(delete_bone, Events::DeleteBone, bone_id, usize);
    event_with_value!(delete_anim, Events::DeleteAnim, anim_id, usize);
    event_with_value!(delete_tex, Events::DeleteTex, tex_id, usize);
    event_with_value!(delete_style, Events::DeleteStyle, style_id, usize);
    event_with_value!(delete_keyframe, Events::DeleteKeyframe, kf_idx, usize);
    event_with_value!(duplicate_anim, Events::DuplicateAnim, anim_idx, usize);
    event_with_value!(copy_bone, Events::CopyBone, bone_id, usize);
    event_with_value!(paste_bone, Events::PasteBone, bone_id, usize);
    event_with_value!(remove_vertex, Events::RemoveVertex, vert_idx, usize);
    event_with_value!(drag_vertex, Events::DragVertex, vert_id, usize);
    event_with_value!(click_vertex, Events::ClickVertex, vert_id, usize);
    event_with_value!(remove_triangle, Events::RemoveTriangle, idx, usize);
    event_with_value!(
        adjust_keyframes_by_fps,
        Events::AdjustKeyframesByFPS,
        fps,
        usize
    );
    event_with_value!(
        remove_keyframes_by_frame,
        Events::RemoveKeyframesByFrame,
        frame,
        i32
    );
    event_with_value!(
        toggle_showing_mesh,
        Events::ToggleShowingMesh,
        visible,
        usize
    );
    event_with_value!(select_bind, Events::SelectBind, idx, i32);
    event_with_value!(
        toggle_binding_verts,
        Events::ToggleBindingVerts,
        toggle,
        usize
    );
    event_with_value!(
        toggle_setting_ik_target,
        Events::ToggleSettingIkTarget,
        idx,
        i32
    );
    event_with_value!(
        toggle_anim_panel_open,
        Events::ToggleAnimPanelOpen,
        toggle,
        usize
    );
    event_with_value!(toggle_ik_folded, Events::ToggleIkFolded, toggle, usize);
    event_with_value!(
        toggle_meshdef_folded,
        Events::ToggleMeshdefFolded,
        toggle,
        usize
    );
    event_with_value!(
        toggle_effects_folded,
        Events::ToggleEffectsFolded,
        toggle,
        usize
    );
    event_with_value!(save_edited_bone, Events::SaveEditedBone, bone_idx, usize);
    event_with_value!(save_bone, Events::SaveBone, bone_idx, usize);
    event_with_value!(toggle_baking_ik, Events::ToggleBakingIk, toggle, usize);
    event_with_value!(toggle_exclude_ik, Events::ToggleExcludeIk, toggle, usize);
    event_with_value!(
        set_export_img_format,
        Events::SetExportImgFormat,
        idx,
        usize
    );
    event_with_value!(copy_keyframe, Events::CopyKeyframe, idx, usize);
    event_with_value!(
        toggle_onion_layers,
        Events::ToggleOnionLayers,
        toggle,
        usize
    );

    pub fn open_modal(&mut self, loc_headline: &str, forced: bool) {
        self.events.push(Events::OpenModal);
        self.values.push(if forced { 1. } else { 0. });
        self.str_values.push(loc_headline.to_string());
    }

    pub fn select_bone(&mut self, bone_id: usize, from_renderer: bool) {
        self.events.push(Events::SelectBone);
        self.values.push(bone_id as f32);
        self.str_values.push(if from_renderer {
            "t".to_string()
        } else {
            "f".to_string()
        });
    }

    pub fn rename_bone(&mut self, bone_idx: usize, new_name: String) {
        self.events.push(Events::RenameBone);
        self.values.push(bone_idx as f32);
        self.str_values.push(new_name);
    }

    pub fn open_polar_modal(&mut self, polar_id: PolarId, headline: String) {
        self.events.push(Events::OpenPolarModal);
        self.values.push((polar_id as usize) as f32);
        self.str_values.push(headline);
    }

    pub fn toggle_pointer_on_ui(&mut self, toggle: bool) {
        self.events.push(Events::PointerOnUi);
        self.values.push(if toggle { 1. } else { 0. });
        self.str_values.push("".to_string());
    }

    pub fn drag_bone(&mut self, is_above: bool, point_id: usize, drag_id: usize) {
        self.events.push(Events::DragBone);
        self.values.push(if is_above { 1. } else { 0. });
        self.values.push(point_id as f32);
        self.values.push(drag_id as f32);
    }

    pub fn set_keyframe_frame(&mut self, keyframe: usize, frame: usize) {
        self.events.push(Events::SetKeyframeFrame);
        self.values.push(keyframe as f32);
        self.values.push(frame as f32);
    }

    pub fn set_all_keyframe_frame(&mut self, from_frame: usize, to_frame: usize) {
        self.events.push(Events::SetAllKeyframesFrame);
        self.values.push(from_frame as f32);
        self.values.push(to_frame as f32);
    }

    pub fn rename_animation(&mut self, anim_idx: usize, name: String) {
        self.events.push(Events::RenameAnim);
        self.values.push(anim_idx as f32);
        self.str_values.push(name);
    }

    pub fn rename_style(&mut self, style_idx: usize, name: String) {
        self.events.push(Events::RenameStyle);
        self.values.push(style_idx as f32);
        self.str_values.push(name);
    }

    pub fn toggle_anim_playing(&mut self, anim_idx: usize, playing: bool) {
        self.events.push(Events::ToggleAnimPlaying);
        self.values.push(anim_idx as f32);
        self.values.push(if playing { 1. } else { 0. });
    }

    pub fn toggle_style_active(&mut self, style_idx: usize, toggle: bool) {
        self.events.push(Events::ToggleStyleActive);
        self.values.push(style_idx as f32);
        self.values.push(if toggle { 1. } else { 0. });
    }

    pub fn move_style(&mut self, point_idx: usize, drag_idx: usize) {
        self.events.push(Events::MoveStyle);
        self.values.push(point_idx as f32);
        self.values.push(drag_idx as f32);
    }

    pub fn migrate_texture(&mut self, point_idx: usize, drag_idx: usize) {
        self.events.push(Events::MigrateTexture);
        self.values.push(point_idx as f32);
        self.values.push(drag_idx as f32);
    }

    pub fn rename_texture(&mut self, tex_idx: usize, new_name: String) {
        self.events.push(Events::RenameTex);
        self.values.push(tex_idx as f32);
        self.str_values.push(new_name);
    }

    pub fn move_texture(&mut self, old_idx: usize, new_idx: usize) {
        self.events.push(Events::MoveTexture);
        self.values.push(new_idx as f32);
        self.values.push(old_idx as f32);
    }

    pub fn toggle_bone_folded(&mut self, bone_idx: usize, folded: bool) {
        self.events.push(Events::ToggleBoneFolded);
        self.values.push(bone_idx as f32);
        self.values.push(if folded { 1. } else { 0. });
    }

    pub fn edit_bone(
        &mut self,
        bone_id: i32,
        element: &AnimElement,
        value: f32,
        value_str: &str,
        anim_id: usize,
        anim_frame: i32,
    ) {
        self.events.push(Events::EditBone);
        self.values.push(bone_id as f32);
        self.values.push((element.clone() as usize) as f32);
        self.values.push(value as f32);
        self.values.push(anim_id as f32);
        self.values.push(anim_frame as f32);
        self.str_values.push(value_str.to_string());
    }

    pub fn set_bone_texture(&mut self, bone_id: usize, tex: String) {
        self.events.push(Events::SetBoneTexture);
        self.values.push(bone_id as f32);
        self.str_values.push(tex);
    }

    pub fn adjust_vertex(&mut self, pos_x: f32, pos_y: f32) {
        self.events.push(Events::AdjustVertex);
        self.values.push(pos_x as f32);
        self.values.push(pos_y as f32);
    }

    pub fn edit_camera(&mut self, pos_x: f32, pos_y: f32, zoom: f32) {
        self.events.push(Events::EditCamera);
        self.values.push(pos_x as f32);
        self.values.push(pos_y as f32);
        self.values.push(zoom as f32);
    }

    pub fn toggle_bone_ik_disabled(&mut self, bone_idx: usize, toggle: bool) {
        self.events.push(Events::ToggleIkDisabled);
        self.values.push(bone_idx as f32);
        self.values.push(if toggle { 1. } else { 0. });
    }

    pub fn toggle_bind_pathing(&mut self, bind_idx: usize, toggle: bool) {
        self.events.push(Events::ToggleBindPathing);
        self.values.push(bind_idx as f32);
        self.values.push(if toggle { 1. } else { 0. });
    }

    pub fn set_bind_weight(&mut self, vert_idx: usize, weight: f32) {
        self.events.push(Events::SetBindWeight);
        self.values.push(vert_idx as f32);
        self.values.push(weight);
    }

    pub fn open_file_err_modal(&mut self, err: String) {
        self.events.push(Events::OpenFileErrModal);
        self.values.push(-1.);
        self.str_values.push(err);
    }

    pub fn select_anim_frame(&mut self, frame: usize, show_panel: bool) {
        self.events.push(Events::SelectAnimFrame);
        self.values.push(frame as f32);
        self.values.push(if show_panel { 1. } else { 0. });
    }

    pub fn delete_keyframe_line(&mut self, bone_id: usize, element: &AnimElement) {
        self.events.push(Events::DeleteKeyframeLine);
        self.values.push(bone_id as f32);
        self.values.push((element.clone() as usize) as f32);
    }

    pub fn set_export_clear_color(&mut self, r: f32, g: f32, b: f32) {
        self.events.push(Events::SetExportClearColor);
        self.values.push(r as f32);
        self.values.push(g as f32);
        self.values.push(b as f32);
    }

    pub fn update_keyframe_transition(
        &mut self,
        frame: i32,
        is_in: bool,
        handle: Vec2,
        preset: i32,
    ) {
        self.events.push(Events::UpdateKeyframeTransition);
        self.values.push(frame as f32);
        self.values.push(if is_in { 1. } else { 0. });
        self.values.push(handle.x);
        self.values.push(handle.y);
        self.values.push(preset as f32);
    }
}

#[derive(Default, Clone)]
pub struct SelectionState {
    pub bone_idx: usize,
    pub bone_ids: Vec<i32>,
    pub style: i32,
    pub bind: i32,
    pub anim: usize,
    pub anim_frame: i32,
}

#[derive(Default)]
pub struct Shared {
    pub armature: Armature,
    pub input: InputStates,
    pub ui: Ui,
    pub undo_states: UndoStates,
    pub renderer: Renderer,
    pub events: EventState,
    pub camera: Camera,
    pub selections: SelectionState,
    pub edit_mode: EditMode,
    pub config: Config,
    pub copy_buffer: CopyBuffer,
    pub last_autosave: f32,
    pub screenshot_res: Vec2,
}

// generate non-clashing id
pub fn generate_id(ids: Vec<i32>) -> i32 {
    let mut idx = 0;
    while idx == does_id_exist(idx, ids.clone()) {
        idx += 1;
    }
    return idx;
}

fn does_id_exist(id: i32, ids: Vec<i32>) -> i32 {
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

fn default_tint() -> TintColor {
    TintColor::new(1., 1., 1., 1.)
}

fn is_neg_one(value: &i32) -> bool {
    *value == -1
}

fn is_max(value: &f32) -> bool {
    *value == f32::MAX
}

fn is_color_empty(value: &Color) -> bool {
    *value == Color::new(0, 0, 0, 0)
}

fn are_verts_empty(value: &Vec<Vertex>) -> bool {
    *value == vec![]
}

fn are_indices_empty<T: std::cmp::PartialEq<Vec<u32>>>(value: &T) -> bool {
    *value == vec![]
}

fn is_tint_white<T: std::cmp::PartialEq<TintColor>>(value: &T) -> bool {
    *value == TintColor::new(1., 1., 1., 1.)
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

pub fn f32_as_bool(value: f32) -> bool {
    value == 1.
}

pub fn bool_as_f32(value: bool) -> f32 {
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
pub fn color_path() -> std::path::PathBuf {
    directories_next::ProjectDirs::from("com", "retropaint", "skelform")
        .map(|proj_dirs| proj_dirs.data_dir().join("colors.json"))
        .unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn recents_path() -> std::path::PathBuf {
    directories_next::ProjectDirs::from("com", "retropaint", "skelform")
        .map(|proj_dirs| proj_dirs.data_dir().join("recent_files.json"))
        .unwrap()
}
