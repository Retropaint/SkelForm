//! Easily-accessible and frequently-shared data.

use std::{
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

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
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
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

    pub scroll: Vec2,

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

    // id to identify actions for polar (yes-no) dialog
    pub polar_id: String,
    pub polar_headline: String,

    pub modal_headline: String,

    pub anim: UiAnim,

    // the initial value of what is being edited via input
    pub edit_value: Option<String>,
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
        let painter = ui.painter();
        painter.rect_stroke(
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

    pub timeline_offset: f32,
    pub dragged_keyframe: usize,
    pub images: Vec<egui::TextureHandle>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Bone {
    pub id: i32,
    pub name: String,
    pub parent_id: i32,
    pub tex_idx: i32,

    pub vertices: Vec<Vertex>,

    pub pivot: Vec2,

    /// used to properly offset bone's movement to counteract it's parent
    pub parent_rot: f32,

    pub rot: f32,
    pub scale: Vec2,
    pub pos: Vec2,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Armature {
    pub bones: Vec<Bone>,
    pub animations: Vec<Animation>,

    pub textures: Vec<Texture>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Texture {
    pub size: Vec2,

    #[serde(skip)]
    pub pixels: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Animation {
    pub name: String,
    pub fps: i32,
    pub keyframes: Vec<Keyframe>,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct Keyframe {
    pub frame: i32,
    pub bones: Vec<AnimBone>,
}

#[derive(PartialEq, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct AnimBone {
    pub id: i32,
    pub pos: Vec2,
    pub scale: Vec2,
    pub rot: f32,

    #[serde(skip)]
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
pub struct AnimField {
    pub element: AnimElement,

    // If the next field is related to this, connect is true.
    //
    // Example: Color is a vec4 value (RGBA), so the first field
    // is for RG, while second is for BA. The first field's
    // connect is true, while the second one's is false as it does not connect
    // to the field after it.
    //
    // This can be chained to have as many even-numbered vecs as possible.
    pub connect: bool,

    pub value: Vec2,

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
        let id = self.selected_bone().id;
        for b in &mut self.selected_keyframe_mut().unwrap().bones {
            if b.id == id {
                return Some(b);
            }
        }
        None
    }

    pub fn selected_anim_bone(&self) -> Option<&AnimBone> {
        let id = self.selected_bone().id;
        for b in &self.selected_keyframe().unwrap().bones {
            if b.id == id {
                return Some(b);
            }
        }
        None
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

    pub fn selected_bone(&self) -> &Bone {
        &self.armature.bones[self.selected_bone_idx]
    }

    pub fn selected_bone_mut(&mut self) -> &mut Bone {
        &mut self.armature.bones[self.selected_bone_idx]
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
                    let (prev, next, total_frames, current_frame, _, _) = self
                        .find_connecting_frames(
                            b.id,
                            $element,
                            $default,
                            self.ui.anim.selected_frame,
                        );
                    Tweener::linear(prev, next, total_frames).move_to(current_frame)
                }};
            }

            // interpolate!
            b.pos += interpolate!(AnimElement::Position, Vec2::ZERO);
            b.rot += interpolate!(AnimElement::Rotation, Vec2::ZERO).x;
            b.scale *= interpolate!(AnimElement::Scale, Vec2::new(1., 1.));
            b.pivot *= interpolate!(AnimElement::Pivot, Vec2::new(1., 1.));
        }

        bones
    }

    pub fn find_connecting_frames(
        &self,
        bone_id: i32,
        element: AnimElement,
        default: Vec2,
        frame: i32,
    ) -> (Vec2, Vec2, i32, i32, i32, i32) {
        let mut prev: Option<Vec2> = None;
        let mut next: Option<Vec2> = None;
        let mut start_frame = 0;
        let mut end_frame = 0;

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
            start_frame,
            end_frame,
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
            let parent_id = self.selected_bone().parent_id;
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
            bone: self.selected_bone().clone(),
            id: self.selected_bone().id,
            ..Default::default()
        });
    }

    pub fn edit_bone(&mut self, edit_mode: i32, value: Vec2) {
        if self.armature.bones[self.selected_bone_idx].tex_idx == -1 {
            return;
        }

        let mut element = crate::AnimElement::Position;

        match edit_mode {
            0 => {
                element = crate::AnimElement::Position;
                if !self.is_animating() {
                    self.selected_bone_mut().pos = value;
                }
            }
            1 => {
                element = crate::AnimElement::Rotation;
                if !self.is_animating() {
                    self.selected_bone_mut().rot = value.x;
                }
            }
            2 => {
                element = crate::AnimElement::Scale;
                if !self.is_animating() {
                    self.selected_bone_mut().scale = value;
                }
            }
            3 => {
                element = crate::AnimElement::Pivot;
                if !self.is_animating() {
                    self.selected_bone_mut().pivot = value;
                }
            }
            _ => {}
        }

        if self.is_animating() {
            self.check_if_in_keyframe(self.selected_bone().id);
            self.selected_anim_bone_mut()
                .unwrap()
                .set_field(&element, value);
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

    pub fn select_anim(&mut self, idx: usize) {
        self.anim.selected = idx;
        self.anim.selected_frame = 0;
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
