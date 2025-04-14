//! Easily-accessible and frequently-shared data.

use std::{
    fmt,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
};

use tween::Tweener;
use wgpu::BindGroup;
use winit::keyboard::KeyCode;

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

#[derive(PartialEq)]
pub enum Animating {
    BonePos,
    RotPos,
    ScalePos,
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

    pub anim: UiAnim,

    // the initial value of what is being edited via input
    edit_value: Option<String>,
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
    pub timeline_offset: f32,
    pub dragged_keyframe: usize,
    pub images: Vec<egui::TextureHandle>,
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

    pub fields: Vec<AnimField>,

    #[serde(skip)]
    pub pos_top: f32,
    #[serde(skip)]
    pub rot_top: f32,
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

#[derive(PartialEq, serde::Serialize, Clone, Default)]
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

#[derive(Eq, Ord, PartialEq, PartialOrd, serde::Serialize, Clone, Default, Debug)]
pub enum AnimElement {
    #[default]
    Position,

    Rotation,
    Scale,
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

    pub fn animate(&self, _anim_idx: usize, frame: i32) -> Vec<Bone> {
        let mut bones = self.armature.bones.clone();

        // ignore if this animation has no keyframes
        let kf_len = self.selected_animation().keyframes.len();
        if kf_len == 0 {
            return bones;
        }

        for b in &mut bones {
            let mut start_kf: Option<&Keyframe> = None;
            let mut end_kf: Option<&Keyframe> = None;
            let mut start_bone: Option<&AnimBone> = None;
            let mut end_bone: Option<&AnimBone> = None;

            // get the frames to interpolate (or otherwise)
            for kf in &self.selected_animation().keyframes {
                for ab in &kf.bones {
                    if ab.id != b.id {
                        continue;
                    }
                    if kf.frame == frame {
                        start_kf = Some(&kf);
                        start_bone = Some(&ab);
                        end_kf = Some(&kf);
                        end_bone = Some(&ab);
                        break;
                    }
                    if kf.frame <= frame
                        || frame < self.selected_animation().keyframes[0].frame && start_kf == None
                    {
                        start_kf = Some(&kf);
                        start_bone = Some(&ab);
                    } else if kf.frame > frame && end_kf == None {
                        end_kf = Some(&kf);
                        end_bone = Some(&ab);
                    }
                }
            }

            // ensure start and end are always pointing somewhere
            if start_kf == None {
                start_kf = end_kf;
                start_bone = end_bone;
            } else if end_kf == None {
                end_kf = start_kf;
                end_bone = start_bone;
            }

            if start_bone == None && end_bone == None {
                return bones;
            }

            let mut total_frames = end_kf.unwrap().frame - start_kf.unwrap().frame;
            // Tweener can't have 0 duration
            if total_frames == 0 {
                total_frames = 1;
            }

            let current_frame = frame - start_kf.unwrap().frame;

            macro_rules! interpolate {
                ($element:expr) => {
                    Tweener::linear(
                        start_bone.unwrap().find_field(&$element),
                        end_bone.unwrap().find_field(&$element),
                        total_frames,
                    )
                    .move_to(current_frame)
                };
            }

            // interpolate!
            b.pos = interpolate!(AnimElement::Position);
            b.rot = interpolate!(AnimElement::Rotation).x;
            b.scale = interpolate!(AnimElement::Scale);
        }

        bones
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

    pub fn singleline_input(&mut self, mut value: f32, ui: &mut egui::Ui) -> f32 {
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            if self.edit_value != None {
                let mut string = self.edit_value.clone().unwrap();
                let input = ui.add_sized([40., 20.], egui::TextEdit::singleline(&mut string));
                input.request_focus();
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
                println!("{}", string);
                let input = ui.add_sized([40., 20.], egui::TextEdit::singleline(&mut string));
                if input.has_focus() {
                    self.edit_value = Some(value.to_string());
                }
            }
        });

        value
    }
}
