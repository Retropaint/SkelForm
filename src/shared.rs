//! Easily-accessible and frequently-shared data.

use std::{
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

pub const RECT_VERT_INDICES: [u32; 6] = [0, 1, 2, 0, 3, 1];

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

    pub fn equal_to(self: &Self, other: Vec2) -> bool {
        return self.x != other.x || self.y != other.y;
    }

    /// For f32 values that need to be passed as Vec2.
    pub fn single(value: f32) -> Vec2 {
        return Vec2::new(value, 0.);
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

#[repr(C)]
#[derive(
    PartialEq, serde::Serialize, serde::Deserialize, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,
)]
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
    pub mouse_left_prev: i32,
    pub mouse: Vec2,

    pub scroll: Vec2,

    pub mouse_prev: Vec2,

    // is mouse on UI?
    pub on_ui: bool,

    pub pressed: Vec<KeyCode>,
}

impl InputStates {
    pub fn is_pressing(&self, key: KeyCode) -> bool {
        for k in &self.pressed {
            if *k == key {
                return true;
            }
        }

        false
    }
}

#[derive(Clone, Default)]
pub struct Ui {
    pub anim: UiAnim,

    pub edit_bar_pos: Vec2,
    pub animate_mode_bar_pos: Vec2,
    pub animate_mode_bar_scale: Vec2,

    pub rename_id: String,
    pub original_name: String,

    // id to identify actions for polar (yes-no) dialog
    pub polar_id: String,
    pub polar_headline: String,

    pub modal_headline: String,
    // if true, the modal can't be closed
    pub forced_modal: bool,

    // the initial value of what is being edited via input
    pub edit_value: Option<String>,

    pub image_modal: bool,

    pub texture_images: Vec<egui::TextureHandle>,

    pub is_removing_textures: bool,

    // camera bar stuff
    pub camera_bar_pos: Vec2,
    pub camera_bar_scale: Vec2,

    pub bone_panel: Option<egui::Rect>,
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

    pub fn draw_rect(&self, rect: egui::Rect, ui: &egui::Ui) {
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1., egui::Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }
}

#[derive(Clone, Default)]
pub struct UiAnim {
    pub selected: usize,
    pub hovering_frame: i32,
    pub selected_frame: i32,
    pub timeline_zoom: f32,
    pub lines_x: Vec<f32>,
    pub playing: bool,
    pub elapsed: Option<std::time::Instant>,

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
    pub is_mesh: bool,

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

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Texture {
    #[serde(default)]
    pub size: Vec2,

    #[serde(skip)]
    pub pixels: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
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
    #[serde(default)]
    pub bones: Vec<AnimBone>,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct AnimBone {
    #[serde(default)]
    pub id: i32,
    #[serde(default)]
    pub fields: Vec<AnimField>,
}

impl AnimBone {
    pub fn set_field(&mut self, element: &AnimElement, value: Vec2) {
        let mut create = true;
        for af in &mut self.fields {
            if af.element == *element {
                create = false;
                af.value = value;
                break;
            }
        }

        if create {
            self.fields.push(AnimField {
                element: element.clone(),
                value,
                ..Default::default()
            });
        }
    }

    pub fn find_field(&self, element: &AnimElement) -> Vec2 {
        for af in &self.fields {
            if af.element == *element {
                return af.value;
            }
        }

        AnimElement::default_of(element)
    }

    pub fn remove_field(&mut self, element: &AnimElement) {
        for i in 0..self.fields.len() {
            let af = &self.fields[i];
            if af.element == *element {
                self.fields.remove(i);
                break;
            }
        }
    }
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub enum Transition {
    #[default]
    Linear,
    Sine,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct AnimField {
    #[serde(default)]
    pub element: AnimElement,

    // If the next field is related to this, connect is true.
    //
    // Example: Color is a vec4 value (RGBA), so the first field
    // is for RG, while second is for BA. The first field's
    // connect is true, while the second one's is false as it does not connect
    // to the field after it.
    //
    // This can be chained to have as many even-numbered vecs as possible.
    #[serde(default)]
    pub connect: bool,

    #[serde(default)]
    pub value: Vec2,

    #[serde(default)]
    pub transition: Transition,

    #[serde(skip)]
    pub label_top: f32,
}

#[derive(
    Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, Clone, Default, Debug,
)]
pub enum AnimElement {
    #[default]
    Position,
    Rotation,
    Scale,
    Pivot,
    Zindex,
}

#[derive(Default, Clone)]
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

#[derive(Default, Clone)]
pub struct Action {
    pub action: ActionEnum,
    pub action_type: ActionType,

    pub id: i32,
    pub animation: Animation,
    pub bone: Bone,
}

impl AnimElement {
    pub fn default_of(element: &AnimElement) -> Vec2 {
        match *element {
            AnimElement::Scale => {
                return Vec2::new(1., 1.);
            }
            _ => Vec2::ZERO,
        }
    }
}

// this allows getting the element name as a string
impl fmt::Display for AnimElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
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
pub struct Shared {
    pub window: Vec2,
    pub dragging: bool,
    pub selected_bone_idx: usize,
    pub armature: Armature,
    pub bind_groups: Vec<BindGroup>,
    pub camera: Camera,
    pub input: InputStates,
    pub egui_ctx: egui::Context,
    pub cursor_icon: egui::CursorIcon,
    pub ui: Ui,
    pub rendered_frames: Vec<RenderedFrame>,
    pub start_time: Option<std::time::Instant>,
    pub editing_bone: bool,

    pub frame: i32,
    pub recording: bool,
    pub done_recording: bool,

    pub undo_actions: Vec<Action>,
    pub redo_actions: Vec<Action>,

    // tracking zoom every frame for smooth effect
    pub current_zoom: f32,

    // should be enum but too lazy atm
    pub edit_mode: i32,

    pub animating: bool,

    /// useful if you don't want to provide an actual bind group during testing
    pub highlight_bindgroup: Option<BindGroup>,
    pub gridline_bindgroup: Option<BindGroup>,
    pub point_bindgroup: Option<BindGroup>,

    pub save_path: String,

    /// triggers debug stuff
    pub debug: bool,
}

// mostly for shorthands for cleaner code
impl Shared {
    pub fn selected_animation(&self) -> &Animation {
        &self.armature.animations[self.ui.anim.selected]
    }

    pub fn selected_animation_mut(&mut self) -> &mut Animation {
        &mut self.armature.animations[self.ui.anim.selected]
    }

    pub fn selected_keyframe(&self) -> Option<&Keyframe> {
        let frame = self.ui.anim.selected_frame;
        for kf in &self.selected_animation().keyframes {
            if kf.frame == frame {
                return Some(kf);
            }
        }
        None
    }

    pub fn selected_keyframe_mut(&mut self) -> Option<&mut Keyframe> {
        let frame = self.ui.anim.selected_frame;
        for kf in &mut self.selected_animation_mut().keyframes {
            if kf.frame == frame {
                return Some(kf);
            }
        }
        None
    }

    pub fn selected_anim_bone_mut(&mut self) -> Option<&mut AnimBone> {
        let id = self.selected_bone().unwrap().id;
        for b in &mut self.selected_keyframe_mut().unwrap().bones {
            if b.id == id {
                return Some(b);
            }
        }
        None
    }

    pub fn selected_anim_bone(&self) -> Option<&AnimBone> {
        let id = self.selected_bone().unwrap().id;
        for b in &self.selected_keyframe().unwrap().bones {
            if b.id == id {
                return Some(b);
            }
        }
        None
    }

    pub fn unselect_everything(&mut self) {
        self.selected_bone_idx = usize::MAX;
    }

    pub fn select_bone(&mut self, idx: usize) {
        self.unselect_everything();
        self.selected_bone_idx = idx;
    }

    pub fn select_frame(&mut self, idx: i32) {
        self.unselect_everything();
        self.ui.anim.selected_frame = idx;
    }

    pub fn sort_keyframes(&mut self) {
        self.selected_animation_mut()
            .keyframes
            .sort_by(|a, b| a.frame.cmp(&b.frame));
    }

    pub fn last_keyframe(&self) -> Option<&Keyframe> {
        self.selected_animation().keyframes.last()
    }

    pub fn keyframe_at(&self, frame: i32) -> Option<&Keyframe> {
        for kf in &self.selected_animation().keyframes {
            if kf.frame == frame {
                return Some(&kf);
            }
        }

        None
    }

    pub fn keyframe_at_mut(&mut self, frame: i32) -> Option<&mut Keyframe> {
        for kf in &mut self.selected_animation_mut().keyframes {
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
        let kf_len = self.selected_animation().keyframes.len();
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
                        Transition::Sine => {
                            Tweener::linear(prev, next, total_frames).move_to(current_frame)
                        }
                        _ => Tweener::linear(prev, next, total_frames).move_to(current_frame),
                    }
                }};
            }

            // interpolate!
            b.pos += interpolate!(AnimElement::Position, Vec2::ZERO);
            b.rot += interpolate!(AnimElement::Rotation, Vec2::ZERO).x;
            b.scale *= interpolate!(AnimElement::Scale, Vec2::new(1., 1.));
            b.pivot *= interpolate!(AnimElement::Pivot, Vec2::new(1., 1.));
            b.zindex += interpolate!(AnimElement::Zindex, Vec2::ZERO).x;
        }

        bones
    }

    pub fn find_connecting_frames(
        &self,
        bone_id: i32,
        element: AnimElement,
        default: Vec2,
        frame: i32,
    ) -> (Vec2, Vec2, i32, i32, Transition) {
        let mut prev: Option<Vec2> = None;
        let mut next: Option<Vec2> = None;
        let mut start_frame = 0;
        let mut end_frame = 0;
        let mut transition: Transition = Transition::Linear;

        // get most previous frame with this element
        for (i, kf) in self.selected_animation().keyframes.iter().enumerate() {
            if self.selected_animation().keyframes[i].frame > frame {
                break;
            }
            for bone in &self.selected_animation().keyframes[i].bones {
                if bone.id != bone_id {
                    continue;
                }

                for f in &bone.fields {
                    if f.element != element {
                        continue;
                    }

                    prev = Some(f.value);
                    start_frame = kf.frame;
                }
            }
        }

        // get first next frame with this element
        for (i, kf) in self.selected_animation().keyframes.iter().enumerate().rev() {
            if self.selected_animation().keyframes[i].frame < frame {
                break;
            }
            for bone in &self.selected_animation().keyframes[i].bones {
                if bone.id != bone_id {
                    continue;
                }

                for f in &bone.fields {
                    if f.element != element {
                        continue;
                    }

                    transition = f.transition.clone();

                    next = Some(f.value);
                    end_frame = kf.frame;
                }
            }
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

    pub fn get_mouse_world(&mut self) -> Vec2 {
        // get mouse in world space
        let mut mouse_world = crate::utils::screen_to_world_space(self.input.mouse, self.window);
        mouse_world.x *= self.window.x / self.window.y;
        mouse_world
    }

    pub fn move_with_mouse(&mut self, value: &Vec2, counter_parent: bool) -> Vec2 {
        let mut mouse = self.get_mouse_world();

        // Counter-act parent's rotation so that translation is global.
        // Only used in bone translation.
        if counter_parent {
            let parent_id = self.selected_bone().unwrap().parent_id;
            if let Some(parent) = self.find_bone(parent_id) {
                mouse = crate::utils::rotate(&mouse, -parent.rot);
            }
        }

        // Upon immediately clicking, track initial values to allow 'dragging'
        if self.input.initial_points.len() == 0 {
            let initial = mouse * self.camera.zoom;
            self.input.initial_points.push(*value - initial);
        }

        (mouse * self.camera.zoom) + self.input.initial_points[0]
    }

    pub fn save_edited_bone(&mut self) {
        self.undo_actions.push(Action {
            action: ActionEnum::Bone,
            action_type: ActionType::Edited,
            bone: self.selected_bone().unwrap().clone(),
            id: self.selected_bone().unwrap().id,
            ..Default::default()
        });
    }

    pub fn edit_bone(&mut self, edit_mode: i32, mut value: Vec2, overwrite: bool) {
        let mut element = crate::AnimElement::Position;
        let mut og_value = value;

        macro_rules! edit {
            ($element:expr, $field:expr) => {
                element = $element;
                og_value = $field;
                if !self.is_animating() {
                    $field = value;
                } else if overwrite {
                    value -= $field;
                }
            };
        }

        macro_rules! edit_f32 {
            ($element:expr, $field:expr) => {
                element = $element;
                og_value = Vec2::single($field);
                if !self.is_animating() {
                    $field = value.x;
                } else if overwrite {
                    value.x -= $field;
                }
            };
        }

        match edit_mode {
            0 => {
                edit!(
                    crate::AnimElement::Position,
                    self.selected_bone_mut().unwrap().pos
                );
            }
            1 => {
                edit_f32!(
                    crate::AnimElement::Position,
                    self.selected_bone_mut().unwrap().rot
                );
            }
            2 => {
                edit!(
                    crate::AnimElement::Scale,
                    self.selected_bone_mut().unwrap().scale
                );
            }
            3 => {
                edit!(
                    crate::AnimElement::Pivot,
                    self.selected_bone_mut().unwrap().pivot
                );
            }
            4 => {
                edit_f32!(
                    crate::AnimElement::Zindex,
                    self.selected_bone_mut().unwrap().zindex
                );
            }
            _ => {}
        }

        if self.is_animating() {
            if self.ui.anim.selected_frame != 0 {
                self.check_first_keyframe(element.clone(), og_value);
            }
            self.check_if_in_keyframe(self.selected_bone().unwrap().id);
            self.selected_anim_bone_mut()
                .unwrap()
                .set_field(&element, value);
            self.sort_keyframes();
        }
    }

    // If editing a keyframe that isn't the first, the original value should be
    // immediately added to the first keyframe if there isn't a previous keyframe
    // from the selected frame.
    fn check_first_keyframe(&mut self, element: AnimElement, value: Vec2) {
        // Add this field to first keyframe if it doesn't exist
        let mut first_field_missing = true;
        for i in (0..self.ui.anim.selected_frame).rev() {
            let kf = self.keyframe_at(i);
            if kf != None {
                for b in &kf.unwrap().bones {
                    if b.id == self.selected_bone().unwrap().id {
                        for f in &b.fields {
                            if f.element == element {
                                first_field_missing = false;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if first_field_missing {
            let id = self.selected_bone().unwrap().id;
            // add keyframe if it doesn't exist
            if self.selected_animation_mut().keyframes.len() == 0
                || self.selected_animation_mut().keyframes[0].frame != 0
            {
                self.selected_animation_mut().keyframes.push(Keyframe {
                    frame: 0,
                    bones: vec![AnimBone {
                        id: id,
                        fields: vec![AnimField {
                            element: element.clone(),
                            connect: false,
                            value,
                            label_top: 0.,
                            transition: Transition::Linear,
                        }],
                    }],
                });

            // else just add the field
            } else if self.selected_animation_mut().keyframes[0].frame == 0 {
                let mut has_bone = false;
                for b in &mut self.selected_animation_mut().keyframes[0].bones {
                    if b.id == id {
                        b.fields.push(AnimField {
                            element: element.clone(),
                            connect: false,
                            value,
                            label_top: 0.,
                            transition: Transition::Linear,
                        });
                        has_bone = true;
                    }
                }
                if !has_bone {
                    self.selected_animation_mut().keyframes[0]
                        .bones
                        .push(AnimBone {
                            id,
                            fields: vec![AnimField {
                                element: element.clone(),
                                connect: false,
                                value,
                                label_top: 0.,
                                transition: Transition::Linear,
                            }],
                        })
                }
            }
        }
    }

    fn check_if_in_keyframe(&mut self, id: i32) {
        let frame = self.ui.anim.selected_frame;
        // check if this keyframe exists
        let kf = self
            .selected_animation()
            .keyframes
            .iter()
            .position(|k| k.frame == frame);

        if kf == None {
            // create new keyframe
            self.selected_animation_mut()
                .keyframes
                .push(crate::Keyframe {
                    frame,
                    bones: vec![AnimBone {
                        id,
                        ..Default::default()
                    }],
                    ..Default::default()
                });
            self.sort_keyframes();
        } else {
            // check if this bone is in keyframe
            let idx = self.selected_animation().keyframes[kf.unwrap()]
                .bones
                .iter()
                .position(|bone| bone.id == id);

            if idx == None {
                // create anim bone
                self.selected_animation_mut().keyframes[kf.unwrap()]
                    .bones
                    .push(AnimBone {
                        id,
                        ..Default::default()
                    });
            }
        }
    }

    pub fn is_animating(&self) -> bool {
        self.animating && self.ui.anim.selected != usize::MAX
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
