//! DragonBones 5.x JSON importer. See `docs/DRAGONBONES_IMPORT.md`.
//!
//! Turns `<name>_ske.json` + `<name>_tex.json`/`.png` into a regular SkelForm `Armature`.
//! Pure: no file or GPU access (the caller loads files and uploads textures).
//!
//! Conventions: DragonBones is Y-down with clockwise degrees; SkelForm is Y-up with radians,
//! so `y` and rotations are negated. Values DragonBones reads with its own defaults follow
//! the reference parser (DragonBonesCPP `JSONDataParser`), not the format doc, where they
//! differ (eg: a frame without `tweenEasing` holds, it doesn't tween).

use crate::renderer::construction;
use crate::shared::*;
use crate::utils;
use serde_json::Value;
use std::collections::HashMap;

/// One texture atlas page: its JSON and PNG bytes.
pub struct DbAtlas {
    pub json: String,
    pub png: Vec<u8>,
}

pub struct Imported {
    pub armature: Armature,
    /// Things that couldn't be carried over exactly (shown to the user after import).
    pub warnings: Vec<String>,
    /// DragonBones slot name → id of the bone its displays are drawn on.
    pub slot_bones: Vec<(String, i32)>,
}

const TAU: f32 = std::f32::consts::TAU;
const PI: f32 = std::f32::consts::PI;

// ---------------------------------------------------------------- small JSON helpers

/// A boolean that defaults to true (DragonBones' inherit flags).
fn flag(v: &Value, key: &str) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(true)
}

fn num(v: &Value, key: &str, default: f32) -> f32 {
    v.get(key)
        .and_then(|x| x.as_f64())
        .map(|x| x as f32)
        .unwrap_or(default)
}

fn int(v: &Value, key: &str, default: i64) -> i64 {
    v.get(key).and_then(|x| x.as_i64()).unwrap_or(default)
}

fn string<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("")
}

fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

/// A DragonBones transform: (x, y, rotation rad, skew rad, scale x, scale y), Y-down.
#[derive(Clone, Copy, Debug)]
struct DbTransform {
    x: f32,
    y: f32,
    rot: f32,
    skew: f32,
    sx: f32,
    sy: f32,
}

impl DbTransform {
    fn parse(v: Option<&Value>) -> DbTransform {
        let Some(v) = v else {
            return DbTransform {
                x: 0.,
                y: 0.,
                rot: 0.,
                skew: 0.,
                sx: 1.,
                sy: 1.,
            };
        };
        // like JSONDataParser::_parseTransform: rotate/skew win over skX/skY
        let (rot, skew) = if v.get("rotate").is_some() || v.get("skew").is_some() {
            (
                num(v, "rotate", 0.).to_radians(),
                num(v, "skew", 0.).to_radians(),
            )
        } else {
            let skx = num(v, "skX", 0.).to_radians();
            let sky = num(v, "skY", 0.).to_radians();
            (sky, skx - sky)
        };
        let mut t = DbTransform {
            x: num(v, "x", 0.),
            y: num(v, "y", 0.),
            rot,
            skew,
            sx: num(v, "scX", 1.),
            sy: num(v, "scY", 1.),
        };
        // a 180° skew is a mirror: the same matrix as a negative y scale
        if (normalize(t.skew).abs() - PI).abs() < 1e-3 {
            t.sy = -t.sy;
            t.skew = 0.;
        }
        t
    }
}

/// 2D affine matrix `x' = a·x + c·y + tx`, `y' = b·x + d·y + ty` (DragonBones layout).
#[derive(Clone, Copy, Debug)]
struct Mat {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    tx: f32,
    ty: f32,
}

impl Mat {
    fn from_slice(s: &[Value]) -> Option<Mat> {
        let f = |i: usize| s.get(i).and_then(|x| x.as_f64()).map(|x| x as f32);
        Some(Mat {
            a: f(0)?,
            b: f(1)?,
            c: f(2)?,
            d: f(3)?,
            tx: f(4)?,
            ty: f(5)?,
        })
    }

    /// DragonBones `Transform::toMatrix` for a SkelForm world transform (Y flipped).
    fn from_skelform(b: &Bone) -> Mat {
        let (sin, cos) = (-b.rot).sin_cos();
        Mat {
            a: cos * b.scale.x,
            b: sin * b.scale.x,
            c: -sin * b.scale.y,
            d: cos * b.scale.y,
            tx: b.pos.x,
            ty: -b.pos.y,
        }
    }

    fn apply(&self, p: Vec2) -> Vec2 {
        Vec2::new(
            self.a * p.x + self.c * p.y + self.tx,
            self.b * p.x + self.d * p.y + self.ty,
        )
    }

    fn inverse(&self) -> Mat {
        let det = self.a * self.d - self.b * self.c;
        let det = if det.abs() < 1e-12 { 1e-12 } else { det };
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Mat {
            a,
            b,
            c,
            d,
            tx: -(a * self.tx + c * self.ty),
            ty: -(b * self.tx + d * self.ty),
        }
    }

    fn close(&self, o: &Mat) -> bool {
        let lin = [self.a - o.a, self.b - o.b, self.c - o.c, self.d - o.d];
        lin.iter().all(|x| x.abs() < 1e-3)
            && (self.tx - o.tx).abs() < 0.05
            && (self.ty - o.ty).abs() < 0.05
    }
}

// ---------------------------------------------------------------- easing

/// A frame's easing towards the next frame, as DragonBones defines it.
#[derive(Clone, Debug)]
enum DbEase {
    Hold,
    Linear,
    /// `tweenEasing` quad in (< 0), quad out (0..1] or quad in-out (> 1)
    Quad(f32),
    /// `curve` control points
    Curve(Vec<f32>),
}

impl DbEase {
    fn parse(frame: &Value) -> DbEase {
        if let Some(curve) = frame.get("curve").and_then(|c| c.as_array()) {
            let pts: Vec<f32> = curve
                .iter()
                .filter_map(|x| x.as_f64())
                .map(|x| x as f32)
                .collect();
            if pts.len() >= 4 {
                return DbEase::Curve(pts);
            }
        }
        match frame.get("tweenEasing").and_then(|x| x.as_f64()) {
            None => DbEase::Hold,
            Some(e) if e == 0. => DbEase::Linear,
            Some(e) => DbEase::Quad(e as f32),
        }
    }

    /// Eased progress (DragonBones `_getEasingValue` / curve sampling).
    fn value(&self, p: f32) -> f32 {
        match self {
            DbEase::Hold => 0.,
            DbEase::Linear => p,
            DbEase::Quad(e) => {
                let (curve, e) = if *e < 0. {
                    (p * p, -e)
                } else if *e <= 1. {
                    (1. - (1. - p) * (1. - p), *e)
                } else {
                    (0.5 * (1. - (p * PI).cos()), e - 1.)
                };
                (curve - p) * e + p
            }
            DbEase::Curve(pts) => curve_value(pts, p),
        }
    }

    /// SkelForm handles for this easing, or None if it can't be expressed as one cubic
    /// (then the segment gets sampled).
    fn handles(&self) -> Option<(Vec2, Vec2)> {
        let v = Vec2::new;
        Some(match self {
            DbEase::Hold => (v(999., 999.), v(999., 999.)),
            DbEase::Linear => utils::interp_preset(HandlePreset::Linear),
            // quad in/out are quadratic in p, so a cubic with linear x represents them exactly
            DbEase::Quad(e) if *e < 0. => {
                let a = -e;
                (v(1. / 3., (1. - a) / 3.), v(2. / 3., (2. - a) / 3.))
            }
            DbEase::Quad(e) if *e <= 1. => (v(1. / 3., (1. + e) / 3.), v(2. / 3., (2. + e) / 3.)),
            // cosine in-out: approximated by blending towards a sine in-out cubic
            DbEase::Quad(e) => {
                let k = (e - 1.).min(1.);
                let lerp = |a: f32, b: f32| a + (b - a) * k;
                (
                    v(lerp(1. / 3., 0.37), lerp(1. / 3., 0.)),
                    v(lerp(2. / 3., 0.63), lerp(2. / 3., 1.)),
                )
            }
            DbEase::Curve(pts) if pts.len() == 4 => (v(pts[0], pts[1]), v(pts[2], pts[3])),
            DbEase::Curve(_) => return None,
        })
    }
}

/// Piecewise cubic easing curve, as DragonBones reads it: [c1x,c1y, c2x,c2y, (ax,ay, c1x,..)..]
/// from (0,0) to (1,1). Solves x for `t` by bisection.
fn curve_value(pts: &[f32], t: f32) -> f32 {
    let mut segments = vec![];
    let mut start = (0., 0.);
    let mut i = 0;
    while i + 4 <= pts.len() {
        let c1 = (pts[i], pts[i + 1]);
        let c2 = (pts[i + 2], pts[i + 3]);
        let end = if i + 6 <= pts.len() {
            (pts[i + 4], pts[i + 5])
        } else {
            (1., 1.)
        };
        segments.push((start, c1, c2, end));
        start = end;
        i += 6;
    }
    let bez = |a: f32, b: f32, c: f32, d: f32, u: f32| {
        let l = 1. - u;
        l * l * l * a + 3. * l * l * u * b + 3. * l * u * u * c + u * u * u * d
    };
    for (s, c1, c2, e) in &segments {
        if t > e.0 && e.0 < 1. {
            continue;
        }
        let (mut lo, mut hi) = (0., 1.);
        for _ in 0..30 {
            let mid = (lo + hi) * 0.5;
            if bez(s.0, c1.0, c2.0, e.0, mid) < t {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        return bez(s.1, c1.1, c2.1, e.1, (lo + hi) * 0.5);
    }
    t
}

// ---------------------------------------------------------------- timelines

/// A parsed timeline: frames at absolute positions with their values and easings.
struct Track {
    frames: Vec<(i32, Vec<f32>, DbEase)>,
    /// value the last frame tweens to when looping (the first frame's, or for rotations the
    /// first frame's angle reached the short way, like DragonBones)
    wrap: Vec<f32>,
}

impl Track {
    fn parse(frames: &[Value], mut values: impl FnMut(&Value) -> Vec<f32>) -> Track {
        let mut out = vec![];
        let mut start = 0;
        for f in frames {
            out.push((start, values(f), DbEase::parse(f)));
            start += int(f, "duration", 1).max(0) as i32;
        }
        let wrap = out.first().map(|f| f.1.clone()).unwrap_or_default();
        Track { frames: out, wrap }
    }

    /// Wrap rotations the short way (BoneRotateTimelineState: normalized delta to frame 0).
    fn wrap_short(mut self) -> Track {
        if let (Some(first), Some(last)) = (self.frames.first(), self.frames.last()) {
            self.wrap = vec![last.1[0] + normalize(first.1[0] - last.1[0])];
        }
        self
    }

    /// Value at a frame, like DragonBones plays it (the last frame tweens back to the first
    /// when looping).
    fn eval(&self, frame: f32, duration: i32, looping: bool) -> Vec<f32> {
        let n = self.frames.len();
        let i = self
            .frames
            .iter()
            .rposition(|f| f.0 as f32 <= frame)
            .unwrap_or(0);
        let (start, v0, ease) = &self.frames[i];
        let (end, v1) = if i + 1 < n {
            (self.frames[i + 1].0, &self.frames[i + 1].1)
        } else if looping && n > 1 {
            (duration, &self.wrap)
        } else {
            return v0.clone();
        };
        if end <= *start {
            return v0.clone();
        }
        let p = ease.value(((frame - *start as f32) / (end - start) as f32).clamp(0., 1.));
        v0.iter().zip(v1).map(|(a, b)| a + (b - a) * p).collect()
    }
}

/// Keyframes for one or more SkelForm elements from a track: values are mapped by `map`
/// (DragonBones track values → SkelForm element values). Easing goes on the key that ends
/// a segment (SkelForm reads it from the next key). Segments that can't be expressed as one
/// cubic are sampled every frame.
fn track_keys(
    track: &Track,
    bone_id: i32,
    elements: &[AnimElement],
    duration: i32,
    looping: bool,
    map: &dyn Fn(&[f32]) -> Vec<f32>,
) -> Vec<Keyframe> {
    let mut keys = vec![];
    let mut push = |frame: i32, values: Vec<f32>, handles: (Vec2, Vec2)| {
        for (el, v) in elements.iter().zip(values) {
            keys.push(Keyframe {
                frame,
                bone_id,
                element: el.clone(),
                value: v,
                start_handle: handles.0,
                end_handle: handles.1,
                ..Default::default()
            });
        }
    };
    let linear = utils::interp_preset(HandlePreset::Linear);
    let n = track.frames.len();
    for i in 0..n {
        let (start, values, _) = &track.frames[i];
        let handles = if i == 0 {
            linear
        } else {
            track.frames[i - 1].2.handles().unwrap_or(linear)
        };
        // a previous multi-segment curve is sampled up to here
        if i > 0 && track.frames[i - 1].2.handles().is_none() {
            let prev = track.frames[i - 1].0;
            for f in prev + 1..*start {
                push(f, map(&track.eval(f as f32, duration, looping)), linear);
            }
        }
        push(*start, map(values), handles);
    }

    // SkelForm holds after its last key and loops at the animation's last key, so close the
    // loop explicitly at `duration`
    let (last_start, last_values, last_ease) = &track.frames[n - 1];
    if *last_start < duration {
        let tweens = !matches!(last_ease, DbEase::Hold) && looping && n > 1;
        if tweens && last_ease.handles().is_none() {
            for f in last_start + 1..duration {
                push(f, map(&track.eval(f as f32, duration, looping)), linear);
            }
        }
        if tweens {
            let handles = last_ease.handles().unwrap_or(linear);
            push(duration, map(&track.wrap), handles);
        } else {
            push(duration, map(last_values), linear);
        }
    }
    keys
}

/// Rotations of a rotate track as DragonBones reads them: each frame's angle is made
/// relative to the previous one (shortest path, or `clockwise` turns).
/// See JSONDataParser::_parseBoneRotateFrame.
fn unwrap_rotations(frames: &[Value]) -> Vec<f32> {
    let mut out = vec![];
    let mut prev = 0.;
    let mut prev_clockwise = 0.;
    for (i, f) in frames.iter().enumerate() {
        let mut rot = num(f, "rotate", 0.).to_radians();
        if i > 0 {
            if prev_clockwise == 0. {
                rot = prev + normalize(rot - prev);
            } else {
                if (prev_clockwise > 0. && rot >= prev) || (prev_clockwise < 0. && rot <= prev) {
                    prev_clockwise += if prev_clockwise > 0. { -1. } else { 1. };
                }
                rot = rot + TAU * prev_clockwise;
            }
        }
        prev_clockwise = num(f, "clockwise", num(f, "tweenRotate", 0.));
        prev = rot;
        out.push(rot);
    }
    out
}

fn normalize(mut r: f32) -> f32 {
    while r > PI {
        r -= TAU;
    }
    while r < -PI {
        r += TAU;
    }
    r
}

// ---------------------------------------------------------------- DragonBones pose

/// A DragonBones global transform (`Transform`: x, y, rotation, skew, scale).
#[derive(Clone, Copy, Debug)]
struct Global {
    x: f32,
    y: f32,
    rot: f32,
    skew: f32,
    sx: f32,
    sy: f32,
}

impl Global {
    fn from_local(t: &DbTransform) -> Global {
        Global {
            x: t.x,
            y: t.y,
            rot: t.rot,
            skew: t.skew,
            sx: t.sx,
            sy: t.sy,
        }
    }

    /// Transform::toMatrix
    fn to_matrix(&self) -> Mat {
        Mat {
            a: self.rot.cos() * self.sx,
            b: self.rot.sin() * self.sx,
            c: -(self.rot + self.skew).sin() * self.sy,
            d: (self.rot + self.skew).cos() * self.sy,
            tx: self.x,
            ty: self.y,
        }
    }

    /// Transform::fromMatrix (the sign of the scale it had decides flips)
    fn from_matrix(&mut self, m: &Mat) {
        let (backup_sx, backup_sy) = (self.sx, self.sy);
        let quarter = PI / 4.;
        self.x = m.tx;
        self.y = m.ty;
        self.rot = (m.b / m.a).atan();
        let mut skew_x = (-m.c / m.d).atan();
        self.sx = if self.rot > -quarter && self.rot < quarter {
            m.a / self.rot.cos()
        } else {
            m.b / self.rot.sin()
        };
        self.sy = if skew_x > -quarter && skew_x < quarter {
            m.d / skew_x.cos()
        } else {
            -m.c / skew_x.sin()
        };
        if backup_sx >= 0. && self.sx < 0. {
            self.sx = -self.sx;
            self.rot -= PI;
        }
        if backup_sy >= 0. && self.sy < 0. {
            self.sy = -self.sy;
            skew_x -= PI;
        }
        self.skew = skew_x - self.rot;
    }
}

impl Mat {
    /// Matrix::concat: `self` (a child's local) then `parent`
    fn concat(&self, p: &Mat) -> Mat {
        Mat {
            a: self.a * p.a + self.b * p.c,
            b: self.a * p.b + self.b * p.d,
            c: self.c * p.a + self.d * p.c,
            d: self.c * p.b + self.d * p.d,
            tx: p.a * self.tx + p.c * self.ty + p.tx,
            ty: p.d * self.ty + p.b * self.tx + p.ty,
        }
    }

    fn det(&self) -> f32 {
        self.a * self.d - self.b * self.c
    }
}

struct DbIk {
    root: usize,
    /// the chain's second bone (`chain: 1`), solved with `_computeB`
    bone: Option<usize>,
    target: usize,
    bend_positive: bool,
    weight: f32,
}

/// Computes DragonBones world transforms like the runtime (Bone::_updateGlobalTransformMatrix
/// and IKConstraint, no armature flips).
struct Poser<'a> {
    bones: &'a Vec<DbBone>,
    iks: Vec<DbIk>,
    order: Vec<usize>,
}

impl<'a> Poser<'a> {
    fn new(bones: &'a Vec<DbBone>, iks: Vec<DbIk>) -> Poser<'a> {
        // ArmatureData::sortBones: parents first, and a constraint root after its target
        let n = bones.len();
        let mut order: Vec<usize> = vec![];
        let mut placed = vec![false; n];
        let mut guard = 0;
        while order.len() < n && guard < n * n + 10 {
            guard += 1;
            for i in 0..n {
                if placed[i] {
                    continue;
                }
                let parent_ok = bones[i].parent.map(|p| placed[p]).unwrap_or(true);
                let target_ok = iks.iter().filter(|k| k.root == i).all(|k| placed[k.target]);
                if parent_ok && target_ok {
                    placed[i] = true;
                    order.push(i);
                }
            }
        }
        for i in 0..n {
            if !placed[i] {
                order.push(i);
            }
        }
        Poser { bones, iks, order }
    }

    fn update(&self, i: usize, local: &DbTransform, m: &mut Vec<Mat>, g: &mut Vec<Global>) {
        let bone = &self.bones[i];
        let mut global = Global::from_local(local);
        let Some(p) = bone.parent else {
            m[i] = global.to_matrix();
            g[i] = global;
            return;
        };
        let pm = m[p];
        let pg = g[p];
        if bone.inherit_scale {
            if !bone.inherit_rotation {
                global.rot -= pg.rot;
            }
            let mut matrix = global.to_matrix().concat(&pm);
            if !bone.inherit_translation {
                matrix.tx = global.x;
                matrix.ty = global.y;
            }
            global.from_matrix(&matrix);
            m[i] = matrix;
        } else {
            if bone.inherit_translation {
                let (x, y) = (global.x, global.y);
                global.x = pm.a * x + pm.c * y + pm.tx;
                global.y = pm.b * x + pm.d * y + pm.ty;
            }
            if bone.inherit_rotation {
                let mut rot = global.rot + pg.rot + if pg.sx < 0. { PI } else { 0. };
                if pm.det() < 0. {
                    rot -= global.rot * 2.;
                    if bone.inherit_reflection {
                        global.skew += PI;
                    }
                }
                global.rot = rot;
            }
            m[i] = global.to_matrix();
        }
        g[i] = global;
    }

    /// World transforms for the given local transforms; `ik` applies the IK constraints.
    fn pose(&self, locals: &[DbTransform], ik: bool) -> (Vec<Mat>, Vec<Global>) {
        let n = self.bones.len();
        let ident = Mat {
            a: 1.,
            b: 0.,
            c: 0.,
            d: 1.,
            tx: 0.,
            ty: 0.,
        };
        let mut m = vec![ident; n];
        let mut g = vec![Global::from_local(&DbTransform::parse(None)); n];
        let mut solved = vec![false; n];
        for &i in &self.order {
            for k in self.iks.iter().filter(|k| ik && k.root == i) {
                self.update(k.root, &locals[k.root], &mut m, &mut g);
                solved[k.root] = true;
                match k.bone {
                    Some(b) => {
                        self.update(b, &locals[b], &mut m, &mut g);
                        solved[b] = true;
                        self.compute_b(k, &mut m, &mut g);
                    }
                    None => self.compute_a(k, &mut m, &mut g),
                }
            }
            if !solved[i] {
                self.update(i, &locals[i], &mut m, &mut g);
            }
        }
        (m, g)
    }

    /// IKConstraint::_computeA: one bone aims at the target.
    fn compute_a(&self, k: &DbIk, m: &mut Vec<Mat>, g: &mut Vec<Global>) {
        let t = g[k.target];
        let r = &mut g[k.root];
        let mut radian = (t.y - r.y).atan2(t.x - r.x);
        if r.sx < 0. {
            radian += PI;
        }
        r.rot += normalize(radian - r.rot) * k.weight;
        m[k.root] = r.to_matrix();
    }

    /// IKConstraint::_computeB: two bones by the law of cosines.
    fn compute_b(&self, k: &DbIk, m: &mut Vec<Mat>, g: &mut Vec<Global>) {
        let b = k.bone.unwrap();
        let length = self.bones[b].length;
        let ik = g[k.target];
        let mut parent = g[k.root];
        let mut global = g[b];
        let bm = m[b];

        let x = bm.a * length;
        let y = bm.b * length;
        let l_ll = x * x + y * y;
        let l_l = l_ll.sqrt();
        let (mut dx, mut dy) = (global.x - parent.x, global.y - parent.y);
        let l_pp = dx * dx + dy * dy;
        let l_p = l_pp.sqrt();
        let raw_radian = global.rot;
        let raw_parent_radian = parent.rot;
        let raw_radian_a = dy.atan2(dx);

        dx = ik.x - parent.x;
        dy = ik.y - parent.y;
        let l_tt = dx * dx + dy * dy;
        let l_t = l_tt.sqrt();

        let mut radian_a;
        if l_l + l_p <= l_t || l_t + l_l <= l_p || l_t + l_p <= l_l {
            radian_a = (ik.y - parent.y).atan2(ik.x - parent.x);
            if l_l + l_p > l_t && l_p < l_l {
                radian_a += PI;
            }
        } else {
            let h = (l_pp - l_ll + l_tt) / (2. * l_tt);
            let r = (l_pp - h * h * l_tt).max(0.).sqrt() / l_t;
            let hx = parent.x + dx * h;
            let hy = parent.y + dy * h;
            let rx = -dy * r;
            let ry = dx * r;
            let ppr = self.bones[k.root]
                .parent
                .map(|pp| m[pp].det() < 0.)
                .unwrap_or(false);
            if ppr != k.bend_positive {
                global.x = hx - rx;
                global.y = hy - ry;
            } else {
                global.x = hx + rx;
                global.y = hy + ry;
            }
            radian_a = (global.y - parent.y).atan2(global.x - parent.x);
        }

        let dr = normalize(radian_a - raw_radian_a);
        parent.rot = raw_parent_radian + dr * k.weight;
        m[k.root] = parent.to_matrix();
        let current_a = raw_radian_a + dr * k.weight;
        global.x = parent.x + current_a.cos() * l_p;
        global.y = parent.y + current_a.sin() * l_p;
        let mut radian_b = (ik.y - global.y).atan2(ik.x - global.x);
        if global.sx < 0. {
            radian_b += PI;
        }
        global.rot = parent.rot + raw_radian - raw_parent_radian
            + normalize(radian_b - dr - raw_radian) * k.weight;
        m[b] = global.to_matrix();
        g[k.root] = parent;
        g[b] = global;
    }
}

/// SkelForm world transform (pos, rot, scale; Y-up) of a DragonBones global. A 180° skew
/// is a mirror (negative y scale); any other skew is dropped.
fn skelform_world(g: &Global) -> (Vec2, f32, Vec2) {
    let mut sy = g.sy;
    if (normalize(g.skew).abs() - PI).abs() < 1e-3 {
        sy = -sy;
    }
    (Vec2::new(g.x, -g.y), -g.rot, Vec2::new(g.sx, sy))
}

/// SkelForm local transform giving `world` under a parent world (inverse of
/// renderer::inheritance).
fn skelform_local(
    world: (Vec2, f32, Vec2),
    parent: Option<(Vec2, f32, Vec2)>,
) -> (Vec2, f32, Vec2) {
    let Some((pp, pr, ps)) = parent else {
        return world;
    };
    let (wp, wr, ws) = world;
    let facing_left = crate::renderer::is_facing_left(ps);
    let rot = if facing_left { -(wr - pr) } else { wr - pr };
    let pos = utils::rotate(&(wp - pp), -pr) / ps;
    (pos, rot, ws / ps)
}

// ---------------------------------------------------------------- the importer

struct DbBone {
    name: String,
    parent: Option<usize>,
    transform: DbTransform,
    inherit_scale: bool,
    inherit_rotation: bool,
    inherit_translation: bool,
    inherit_reflection: bool,
    length: f32,
}

struct Builder {
    /// DragonBones bone names, reserved so slot bones never take one
    reserved: Vec<String>,
    bones: Vec<Bone>,
    /// DragonBones bone index → SkelForm bone id
    bone_ids: Vec<i32>,
    warnings: Vec<String>,
}

impl Builder {
    fn bone_mut(&mut self, id: i32) -> &mut Bone {
        self.bones.iter_mut().find(|b| b.id == id).unwrap()
    }

    fn bone(&self, id: i32) -> &Bone {
        self.bones.iter().find(|b| b.id == id).unwrap()
    }

    fn warn(&mut self, msg: String) {
        if !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }

    /// `own_name`: this bone is the DragonBones bone of that (reserved) name
    fn new_bone_named(&mut self, name: &str, parent_id: i32, own_name: bool) -> usize {
        let id = self.bones.len() as i32;
        let taken = |b: &Builder, n: &str| {
            b.bones.iter().any(|x| x.name == n) || (!own_name && b.reserved.iter().any(|r| r == n))
        };
        let mut unique = name.to_string();
        let mut n = 2;
        while taken(self, &unique) {
            unique = format!("{name}_{n}");
            n += 1;
        }
        self.bones.push(Bone {
            id,
            name: unique,
            parent_id,
            scale: Vec2::new(1., 1.),
            pivot_scale: Vec2::new(1., 1.),
            tint: TintColor::new(1., 1., 1., 1.),
            ik_target_id: -1,
            ik_family_id: -1,
            physics_id: -1,
            visuals_id: -1,
            group_color: Color::new(0, 0, 0, 0),
            ..Default::default()
        });
        self.bones.len() - 1
    }

    fn new_bone(&mut self, name: &str, parent_id: i32) -> usize {
        self.new_bone_named(name, parent_id, false)
    }
}

/// Where a slot's displays are drawn: the DragonBones bone itself (when it's the bone's only
/// slot) or a child "slot bone".
struct SlotInfo {
    name: String,
    /// id of the bone the slot's displays are drawn on
    carrier: i32,
    displays: Vec<Value>,
    /// Some(i): this carrier draws only display i (the slot's displays need different
    /// placements or meshes, so each gets its own bone); None: it draws every display
    display: Option<usize>,
}

/// Read `<name>_ske.json` and its texture atlas pages (`<name>_tex.json`, or
/// `<name>_tex_0.json`, `_1`, …), each page's image found through its `imagePath`.
#[cfg(not(target_arch = "wasm32"))]
pub fn read_files(ske_path: &std::path::Path) -> Result<(String, Vec<DbAtlas>), String> {
    let read = |p: &std::path::Path| std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()));
    let ske = String::from_utf8_lossy(&read(ske_path)?).to_string();
    let dir = ske_path.parent().unwrap_or(std::path::Path::new("."));
    let file = ske_path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    let base = file
        .strip_suffix("_ske.json")
        .or(file.strip_suffix(".json"))
        .unwrap_or(file);

    let mut pages = vec![dir.join(format!("{base}_tex.json"))];
    pages.extend((0..32).map(|i| dir.join(format!("{base}_tex_{i}.json"))));
    let mut atlases = vec![];
    for page in pages.iter().filter(|p| p.exists()) {
        let json = String::from_utf8_lossy(&read(page)?).to_string();
        let parsed: Value =
            serde_json::from_str(&json).map_err(|e| format!("{}: {e}", page.display()))?;
        let image = match string(&parsed, "imagePath") {
            "" => page.with_extension("png"),
            path => dir.join(path),
        };
        atlases.push(DbAtlas {
            json,
            png: read(&image)?,
        });
    }
    if atlases.is_empty() {
        return Err(format!(
            "no texture atlas found next to {file} (expected {base}_tex.json)"
        ));
    }
    Ok((ske, atlases))
}

pub fn import(ske_json: &str, atlases: &[DbAtlas]) -> Result<Imported, String> {
    let data: Value = serde_json::from_str(ske_json).map_err(|e| format!("invalid JSON: {e}"))?;
    let version = [string(&data, "version"), string(&data, "compatibleVersion")];
    if !version.iter().any(|v| v.starts_with("5.")) {
        return Err(format!(
            "DragonBones data version '{}' is not supported (5.x only).\n\nOlder files can be upgraded with the official converter: npm install -g dragonbones-tools, then db2 -t new",
            version[0]
        ));
    }
    let Some(raw) = data
        .get("armature")
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
    else {
        return Err("no armature in file".into());
    };

    let mut b = Builder {
        reserved: vec![],
        bones: vec![],
        bone_ids: vec![],
        warnings: vec![],
    };
    let armatures = arr(&data, "armature").len();
    if armatures > 1 {
        b.warn(format!(
            "{armatures} armatures in file; imported the first ('{}')",
            string(raw, "name")
        ));
    }
    let frame_rate = int(raw, "frameRate", int(&data, "frameRate", 24)).max(1) as i32;

    // ------------------------------------------------ textures
    let (tex_data, textures) = load_atlases(atlases, &mut b)?;
    let has_tex = |name: &str| textures.iter().any(|t| t.name == name);

    // ------------------------------------------------ bones (DragonBones order, parents first)
    let raw_bones = arr(raw, "bone");
    let names: Vec<&str> = raw_bones.iter().map(|x| string(x, "name")).collect();
    b.reserved = names.iter().map(|n| n.to_string()).collect();
    let db_bones: Vec<DbBone> = raw_bones
        .iter()
        .map(|x| {
            let t = DbTransform::parse(x.get("transform"));
            DbBone {
                name: string(x, "name").to_string(),
                parent: names
                    .iter()
                    .position(|n| *n == string(x, "parent") && !n.is_empty()),
                transform: t,
                inherit_scale: flag(x, "inheritScale"),
                inherit_rotation: flag(x, "inheritRotation"),
                inherit_translation: flag(x, "inheritTranslation"),
                inherit_reflection: flag(x, "inheritReflection"),
                length: num(x, "length", 0.),
            }
        })
        .collect();
    for db in &db_bones {
        // sub-degree skews are editor rounding noise
        if db.transform.skew.abs() > 1f32.to_radians() {
            b.warn(format!(
                "bone '{}' is skewed; skew isn't supported and was dropped",
                db.name
            ));
        }
    }

    // DragonBones world scale (per component): with inheritScale off, a bone ignores its
    // parent's scale. SkelForm always multiplies, so its local scale is world / parent world.
    let world_scale = |own: &dyn Fn(usize) -> Vec2| -> Vec<Vec2> {
        let mut ws: Vec<Vec2> = vec![Vec2::new(1., 1.); db_bones.len()];
        for i in order_parents_first(&db_bones) {
            let s = own(i);
            ws[i] = match db_bones[i].parent {
                Some(p) if db_bones[i].inherit_scale => ws[p] * s,
                _ => s,
            };
        }
        ws
    };
    let rest_own = |i: usize| Vec2::new(db_bones[i].transform.sx, db_bones[i].transform.sy);
    let rest_world = world_scale(&rest_own);

    let slots_raw = arr(raw, "slot");
    let skin = pick_skin(raw, &mut b);
    let displays_of = |slot: &str| -> Vec<Value> {
        skin.map(|s| arr(s, "slot"))
            .unwrap_or(&[])
            .iter()
            .find(|x| string(x, "name") == slot)
            .map(|x| arr(x, "display").to_vec())
            .unwrap_or_default()
    };

    // A slot's displays share one bone when their placements agree relative to the texture
    // size (SkelForm keeps a texture's placement as a pivot) and none is a mesh; otherwise
    // each display gets its own bone
    let display_tex = |d: &Value| -> String {
        if string(d, "path").is_empty() {
            string(d, "name")
        } else {
            string(d, "path")
        }
        .to_string()
    };
    let split = |displays: &Vec<Value>| -> bool {
        if displays.len() < 2 {
            return false;
        }
        if displays.iter().any(|d| string(d, "type") == "mesh") {
            return true;
        }
        let rel = |d: &Value| {
            let t = DbTransform::parse(d.get("transform"));
            let size = textures
                .iter()
                .find(|x| x.name == display_tex(d))
                .map(|x| x.size);
            let size = size
                .filter(|s| s.x > 0. && s.y > 0.)
                .unwrap_or(Vec2::new(1., 1.));
            let pivot = d
                .get("pivot")
                .map(|p| Vec2::new(num(p, "x", 0.5), num(p, "y", 0.5)));
            (
                Vec2::new(t.x, t.y) / size,
                t.rot,
                Vec2::new(t.sx, t.sy),
                pivot,
            )
        };
        let (p0, r0, s0, v0) = rel(&displays[0]);
        displays.iter().any(|d| {
            let (p, r, sc, v) = rel(d);
            (p - p0).mag() > 1e-3 || (r - r0).abs() > 1e-4 || (sc - s0).mag() > 1e-4 || v != v0
        })
    };

    // depth-first: each bone, then its slot bones, then its children (children contiguous)
    b.bone_ids = vec![-1; db_bones.len()];
    let mut slots: Vec<SlotInfo> = vec![];
    fn visit(
        i: usize,
        parent_id: i32,
        db_bones: &Vec<DbBone>,
        rest_world: &Vec<Vec2>,
        slots_raw: &[Value],
        displays_of: &dyn Fn(&str) -> Vec<Value>,
        split: &dyn Fn(&Vec<Value>) -> bool,
        b: &mut Builder,
        slots: &mut Vec<SlotInfo>,
    ) {
        let db = &db_bones[i];
        let idx = b.new_bone_named(&db.name, parent_id, true);
        let t = db.transform;
        let parent_world = db
            .parent
            .map(|p| rest_world[p])
            .unwrap_or(Vec2::new(1., 1.));
        let bone = &mut b.bones[idx];
        bone.pos = Vec2::new(t.x, -t.y);
        bone.rot = -t.rot;
        bone.scale = rest_world[i] / parent_world;
        let id = bone.id;
        b.bone_ids[i] = id;

        let my_slots: Vec<&Value> = slots_raw
            .iter()
            .filter(|s| string(s, "parent") == db.name)
            .collect();
        for s in &my_slots {
            let name = string(s, "name").to_string();
            let displays = displays_of(&name);
            if split(&displays) {
                for di in 0..displays.len() {
                    let bone = b.new_bone(&format!("{name}/{di}"), id);
                    let carrier = b.bones[bone].id;
                    slots.push(SlotInfo {
                        name: name.clone(),
                        carrier,
                        displays: displays.clone(),
                        display: Some(di),
                    });
                }
                continue;
            }
            let carrier = if my_slots.len() == 1 {
                id
            } else {
                let slot_bone = b.new_bone(&name, id);
                b.bones[slot_bone].id
            };
            slots.push(SlotInfo {
                name,
                carrier,
                displays,
                display: None,
            });
        }
        for c in 0..db_bones.len() {
            if db_bones[c].parent == Some(i) {
                visit(
                    c,
                    id,
                    db_bones,
                    rest_world,
                    slots_raw,
                    displays_of,
                    split,
                    b,
                    slots,
                );
            }
        }
    }
    for i in 0..db_bones.len() {
        if db_bones[i].parent.is_none() {
            visit(
                i,
                -1,
                &db_bones,
                &rest_world,
                slots_raw,
                &displays_of,
                &split,
                &mut b,
                &mut slots,
            );
        }
    }

    // ------------------------------------------------ slots: textures, pivots, draw order, tint
    let mut slot_tex: Vec<Vec<String>> = vec![];
    for (z, s) in slots_raw.iter().enumerate() {
        let slot_name = string(s, "name").to_string();
        let carriers: Vec<usize> = (0..slots.len())
            .filter(|i| slots[*i].name == slot_name)
            .collect();
        let Some(&first) = carriers.first() else {
            b.warn(format!("slot '{slot_name}' has no parent bone; skipped"));
            slot_tex.push(vec![]);
            continue;
        };
        let displays = slots[first].displays.clone();
        let mut warnings: Vec<String> = vec![];
        let mut tex_names = vec![];
        for d in &displays {
            let ty = string(d, "type");
            let name = display_tex(d);
            if ty == "armature" || ty == "boundingBox" {
                warnings.push(format!(
                    "slot '{slot_name}': {ty} displays aren't supported; skipped"
                ));
                tex_names.push(String::new());
                continue;
            }
            if !has_tex(&name) {
                warnings.push(format!(
                    "slot '{slot_name}': texture '{name}' not found in the atlas"
                ));
            }
            tex_names.push(name);
        }

        let display_index = int(s, "displayIndex", 0);
        let shown = usize::try_from(display_index)
            .ok()
            .filter(|i| *i < displays.len());
        let tint = s.get("color").map(|c| {
            if ["aO", "rO", "gO", "bO"].iter().any(|k| num(c, k, 0.) != 0.) {
                warnings.push(format!(
                    "slot '{slot_name}': color offsets aren't supported"
                ));
            }
            TintColor::new(
                num(c, "rM", 100.) / 100.,
                num(c, "gM", 100.) / 100.,
                num(c, "bM", 100.) / 100.,
                num(c, "aM", 100.) / 100.,
            )
        });

        for &ci in &carriers {
            let carrier = slots[ci].carrier;
            let own = slots[ci].display;
            let direct = own.is_none() && b.bone_ids.contains(&carrier);

            // placement from this carrier's display, else the shown one (or the first):
            // DragonBones' default pivot is the image centre, like SkelForm's
            let reference = own
                .or(shown)
                .or(if displays.is_empty() { None } else { Some(0) });
            let mut placement: Option<(Vec2, f32, Vec2, Vec2)> = None; // offset, rot, scale, size
            if let Some(ri) = reference {
                let d = &displays[ri];
                let t = DbTransform::parse(d.get("transform"));
                let is_mesh = string(d, "type") == "mesh";
                let size = textures
                    .iter()
                    .find(|x| x.name == tex_names[ri])
                    .map(|x| x.size)
                    .unwrap_or(Vec2::ZERO);
                let pivot = d
                    .get("pivot")
                    .map(|p| Vec2::new(num(p, "x", 0.5), num(p, "y", 0.5)))
                    .unwrap_or(Vec2::new(0.5, 0.5));
                let center = if is_mesh {
                    Vec2::ZERO
                } else {
                    Vec2::new((0.5 - pivot.x) * size.x, (pivot.y - 0.5) * size.y)
                };
                let scale = Vec2::new(t.sx, t.sy);
                let offset = Vec2::new(t.x, -t.y) + utils::rotate(&(center * scale), -t.rot);
                placement = Some((offset, -t.rot, scale, size));
                if t.skew.abs() > 1f32.to_radians() {
                    warnings.push(format!("slot '{slot_name}': display skew isn't supported"));
                }
            }

            let bone = b.bone_mut(carrier);
            bone.zindex = z as i32;
            if let Some(tint) = tint {
                bone.tint = tint;
            }
            // the displayed texture (an empty texture hides it without hiding children)
            bone.tex = match own {
                Some(i) if shown == Some(i) => tex_names[i].clone(),
                Some(_) => String::new(),
                None => shown.map(|i| tex_names[i].clone()).unwrap_or_default(),
            };
            if let Some((offset, rot, scale, size)) = placement {
                if direct {
                    // on the bone itself: the display transform becomes the texture pivot
                    if size.x > 0. && size.y > 0. {
                        bone.pivot_pos = offset / size;
                    }
                    bone.pivot_rot = rot;
                    bone.pivot_scale = scale;
                } else {
                    bone.pos = offset;
                    bone.rot = rot;
                    bone.scale = scale;
                }
            }
        }
        for w in warnings {
            b.warn(w);
        }
        slot_tex.push(tex_names);
    }

    // ------------------------------------------------ bones SkelForm can't compose itself
    // IK chains and bones not inheriting rotation/translation: their SkelForm transforms come
    // from DragonBones' own pose evaluation (setup here, every frame in animations)
    let iks = parse_ik(raw, &db_bones, &mut b);
    let mut baked: Vec<usize> = (0..db_bones.len())
        .filter(|i| !db_bones[*i].inherit_rotation || !db_bones[*i].inherit_translation)
        .collect();
    for k in &iks {
        baked.push(k.root);
        baked.extend(k.bone);
    }
    baked.sort();
    baked.dedup();
    let poser = Poser::new(&db_bones, iks);
    if !baked.is_empty() {
        let rest: Vec<DbTransform> = db_bones.iter().map(|x| x.transform).collect();
        let (_, globals) = poser.pose(&rest, false);
        for (i, (pos, rot, scale)) in baked_locals(&db_bones, &globals, &baked) {
            let bone = b.bone_mut(b.bone_ids[i]);
            bone.pos = pos;
            bone.rot = rot;
            bone.scale = scale;
        }
    }

    // ------------------------------------------------ meshes (needs the setup pose)
    let mut arm = Armature {
        bones: b.bones.clone(),
        ..Default::default()
    };
    let world = setup_world(&arm);
    for s in slots.iter() {
        let mesh = match s.display {
            Some(i) => s.displays.get(i).filter(|d| string(d, "type") == "mesh"),
            None => s.displays.iter().find(|d| string(d, "type") == "mesh"),
        };
        if let Some(d) = mesh {
            import_mesh(d, s, &world, &mut b);
        }
    }
    arm.bones = b.bones.clone();

    // ------------------------------------------------ animations
    let bones_snapshot = b.bones.clone();
    let mut animations = vec![];
    for a in arr(raw, "animation") {
        animations.push(import_animation(
            a,
            frame_rate,
            &db_bones,
            &bones_snapshot,
            &slots,
            slots_raw,
            &slot_tex,
            &world_scale,
            &poser,
            &baked,
            &mut b,
        ));
    }

    // only the textures this armature's displays use (atlases are often shared by every
    // armature in the file)
    let used: Vec<&String> = slot_tex.iter().flatten().collect();
    let textures: Vec<Texture> = textures
        .into_iter()
        .filter(|t| used.contains(&&t.name))
        .collect();
    let tex_data: Vec<TextureData> = tex_data
        .into_iter()
        .filter(|d| textures.iter().any(|t| t.data_id == d.id))
        .collect();

    let mut armature = Armature {
        bones: b.bones,
        animations,
        styles: vec![Style {
            id: 0,
            name: "Default".into(),
            active: true,
            textures,
        }],
        tex_data,
        animated_bones: vec![],
    };
    // bind-posed meshes get their helper bones
    crate::bind_pose::maintain(&mut armature);

    let slot_bones = slots.iter().map(|s| (s.name.clone(), s.carrier)).collect();
    Ok(Imported {
        armature,
        warnings: b.warnings,
        slot_bones,
    })
}

fn order_parents_first(bones: &Vec<DbBone>) -> Vec<usize> {
    let mut order = vec![];
    let mut placed = vec![false; bones.len()];
    while order.len() < bones.len() {
        let before = order.len();
        for i in 0..bones.len() {
            if !placed[i] && bones[i].parent.map(|p| placed[p]).unwrap_or(true) {
                placed[i] = true;
                order.push(i);
            }
        }
        if order.len() == before {
            // cycle or missing parent: place the rest as roots
            for i in 0..bones.len() {
                if !placed[i] {
                    placed[i] = true;
                    order.push(i);
                }
            }
        }
    }
    order
}

fn pick_skin<'a>(raw: &'a Value, b: &mut Builder) -> Option<&'a Value> {
    let skins = arr(raw, "skin");
    if skins.len() > 1 {
        b.warn(format!(
            "{} skins in file; imported the default one only",
            skins.len()
        ));
    }
    skins
        .iter()
        .find(|s| string(s, "name").is_empty() || string(s, "name") == "default")
        .or(skins.first())
}

fn setup_world(arm: &Armature) -> Vec<Bone> {
    let local = arm.bones.clone();
    let mut world = local.clone();
    construction(&mut world, &local);
    world
}

/// Crop every SubTexture out of its atlas page (un-rotating and un-trimming as needed).
fn load_atlases(
    atlases: &[DbAtlas],
    b: &mut Builder,
) -> Result<(Vec<TextureData>, Vec<Texture>), String> {
    let mut tex_data = vec![];
    let mut textures = vec![];
    for page in atlases {
        let json: Value = serde_json::from_str(&page.json)
            .map_err(|e| format!("invalid texture atlas JSON: {e}"))?;
        let img = image::load_from_memory(&page.png)
            .map_err(|e| format!("invalid texture atlas image: {e}"))?;
        if (num(&json, "scale", 1.) - 1.).abs() > 1e-6 {
            b.warn("texture atlas scale isn't 1; textures imported at atlas resolution".into());
        }
        for st in arr(&json, "SubTexture") {
            let name = string(st, "name").to_string();
            if textures.iter().any(|t: &Texture| t.name == name) {
                continue;
            }
            let (x, y) = (num(st, "x", 0.) as u32, num(st, "y", 0.) as u32);
            let (w, h) = (num(st, "width", 0.) as u32, num(st, "height", 0.) as u32);
            let mut region = img.crop_imm(x, y, w.max(1), h.max(1));
            if st.get("rotated").and_then(|v| v.as_bool()).unwrap_or(false) {
                // stored rotated in the atlas; see CCSlot::_updateFrame
                region = region.rotate270();
            }
            let fw = num(st, "frameWidth", -1.);
            let fh = num(st, "frameHeight", -1.);
            if fw > 0. && fh > 0. {
                // trimmed: put it back into its original frame
                let mut frame = image::RgbaImage::new(fw as u32, fh as u32);
                let fx = -num(st, "frameX", 0.) as i64;
                let fy = -num(st, "frameY", 0.) as i64;
                image::imageops::overlay(&mut frame, &region.to_rgba8(), fx, fy);
                region = image::DynamicImage::ImageRgba8(frame);
            }
            let id = tex_data.len() as i32;
            let size = Vec2::new(region.width() as f32, region.height() as f32);
            tex_data.push(TextureData {
                id,
                image: region,
                bind_group: None,
                ui_img: None,
            });
            textures.push(Texture {
                name,
                size,
                data_id: id,
                ..Default::default()
            });
        }
    }
    Ok((tex_data, textures))
}

/// A mesh display on its carrier bone. Meshes whose bonePoses all equal their slotPose are
/// SkelForm-style (classic) skinning; anything else becomes a Bind Pose mesh.
fn import_mesh(d: &Value, s: &SlotInfo, world: &Vec<Bone>, b: &mut Builder) {
    let verts: Vec<f32> = arr(d, "vertices")
        .iter()
        .filter_map(|x| x.as_f64())
        .map(|x| x as f32)
        .collect();
    let uvs: Vec<f32> = arr(d, "uvs")
        .iter()
        .filter_map(|x| x.as_f64())
        .map(|x| x as f32)
        .collect();
    let tris: Vec<u32> = arr(d, "triangles")
        .iter()
        .filter_map(|x| x.as_u64())
        .map(|x| x as u32)
        .collect();
    let n = verts.len() / 2;
    let carrier_id = s.carrier;
    let mut vertices: Vec<Vertex> = (0..n)
        .map(|i| {
            let pos = Vec2::new(verts[i * 2], -verts[i * 2 + 1]);
            let uv = Vec2::new(
                *uvs.get(i * 2).unwrap_or(&0.),
                *uvs.get(i * 2 + 1).unwrap_or(&0.),
            );
            Vertex {
                id: i as u32,
                pos,
                uv,
                init_pos: pos,
                color: Color::new(0, 0, 0, 0),
                add_color: Color::new(0, 0, 0, 0),
                tint: TintColor::new(1., 1., 1., 1.),
                offset_rot: 0.,
            }
        })
        .collect();

    let weights: Vec<f64> = arr(d, "weights")
        .iter()
        .filter_map(|x| x.as_f64())
        .collect();
    let mut binds: Vec<BoneBind> = vec![];
    let mut bind_pose = false;
    if !weights.is_empty() {
        let slot_pose = Mat::from_slice(arr(d, "slotPose")).unwrap_or(Mat {
            a: 1.,
            b: 0.,
            c: 0.,
            d: 1.,
            tx: 0.,
            ty: 0.,
        });
        let bone_pose = arr(d, "bonePose");
        let pose_of = |db_idx: usize| -> Option<Mat> {
            (0..bone_pose.len() / 7)
                .find(|k| bone_pose[k * 7].as_u64() == Some(db_idx as u64))
                .and_then(|k| Mat::from_slice(&bone_pose[k * 7 + 1..k * 7 + 7]))
        };
        // per vertex: [(dragonbones bone index, weight)]
        let mut per_vert: Vec<Vec<(usize, f32)>> = vec![];
        let mut i = 0;
        for _ in 0..n {
            let count = weights.get(i).copied().unwrap_or(0.) as usize;
            i += 1;
            let mut list = vec![];
            for _ in 0..count {
                let bone = weights.get(i).copied().unwrap_or(0.) as usize;
                let w = weights.get(i + 1).copied().unwrap_or(0.) as f32;
                list.push((bone, w));
                i += 2;
            }
            per_vert.push(list);
        }
        let used: Vec<usize> = {
            let mut u: Vec<usize> = per_vert.iter().flatten().map(|x| x.0).collect();
            u.sort();
            u.dedup();
            u
        };

        let classic = used
            .iter()
            .all(|j| pose_of(*j).map(|p| p.close(&slot_pose)).unwrap_or(false));
        let owner_db = b.bone_ids.iter().position(|id| *id == carrier_id);
        if !classic {
            bind_pose = true;
            // rest position in armature space → the carrier's local space at setup
            let owner = world.iter().find(|w| w.id == carrier_id).unwrap().clone();
            let pivot_rot = b.bone(carrier_id).pivot_rot;
            let pivot_scale = b.bone(carrier_id).pivot_scale;
            let mut rebound = false;
            for (vi, v) in vertices.iter_mut().enumerate() {
                let local_db = Vec2::new(verts[vi * 2], verts[vi * 2 + 1]);
                let mut rest = slot_pose.apply(local_db);
                // bound away from the setup pose: re-express at setup via its dominant bone
                if let Some(&(j, _)) = per_vert[vi].iter().max_by(|x, y| x.1.total_cmp(&y.1)) {
                    let setup = b
                        .bone_ids
                        .get(j)
                        .and_then(|id| world.iter().find(|w| w.id == *id))
                        .map(Mat::from_skelform);
                    if let (Some(pose), Some(setup)) = (pose_of(j), setup) {
                        if !pose.close(&setup) {
                            rest = setup.apply(pose.inverse().apply(rest));
                            rebound = true;
                        }
                    }
                }
                let rest = Vec2::new(rest.x, -rest.y);
                v.pos = utils::rotate(&(rest - owner.pos), -(owner.rot + pivot_rot))
                    / (owner.scale * pivot_scale);
                v.init_pos = v.pos;
            }
            if rebound {
                b.warn(format!("mesh in slot '{}' was bound away from the setup pose; its weights are approximated", s.name));
            }
        }

        // linear weights → SkelForm's per-bind weights. The vertex starts on its owner (its
        // share is the carrier's own weight, if it's among the bones), then each bind k lerps
        // by w_k / (owner share + w_1 + … + w_k).
        let mut bind_of: HashMap<usize, usize> = HashMap::new();
        for j in &used {
            if Some(*j) == owner_db {
                continue;
            }
            let Some(&bone_id) = b.bone_ids.get(*j) else {
                continue;
            };
            bind_of.insert(*j, binds.len());
            binds.push(BoneBind {
                bone_id,
                is_path: false,
                verts: vec![],
            });
        }
        for (vi, list) in per_vert.iter().enumerate() {
            let mut acc = list
                .iter()
                .filter(|x| Some(x.0) == owner_db)
                .map(|x| x.1)
                .sum::<f32>();
            let mut ordered: Vec<&(usize, f32)> =
                list.iter().filter(|x| Some(x.0) != owner_db).collect();
            ordered.sort_by_key(|x| bind_of.get(&x.0).copied().unwrap_or(usize::MAX));
            for (j, w) in ordered {
                let Some(&bi) = bind_of.get(j) else { continue };
                acc += w;
                let seq = if acc > 1e-9 { w / acc } else { 0. };
                binds[bi].verts.push(BoneBindVert {
                    id: vi as i32,
                    weight: seq,
                });
            }
        }
        binds.retain(|x| !x.verts.is_empty());
    }

    let bone = b.bone_mut(carrier_id);
    bone.vertices = vertices;
    bone.indices = tris;
    bone.binds = binds;
    bone.verts_edited = true;
    bone.bind_pose = bind_pose;
    if bind_pose {
        // the world-axis pivot offset would sit outside the skinning
        bone.pivot_pos = Vec2::ZERO;
    }
}

/// DragonBones IK constraints, for the pose evaluator (JSONDataParser::_parseIKConstraint:
/// with `chain` > 0 the bone's parent is the root and the bone is solved with it).
fn parse_ik(raw: &Value, db_bones: &Vec<DbBone>, b: &mut Builder) -> Vec<DbIk> {
    let idx = |name: &str| db_bones.iter().position(|x| x.name == name);
    let mut out = vec![];
    for ik in arr(raw, "ik") {
        let (Some(bone), Some(target)) = (idx(string(ik, "bone")), idx(string(ik, "target")))
        else {
            continue;
        };
        let chained = int(ik, "chain", 0) > 0 && db_bones[bone].parent.is_some();
        let (root, bone) = if chained {
            (db_bones[bone].parent.unwrap(), Some(bone))
        } else {
            (bone, None)
        };
        out.push(DbIk {
            root,
            bone,
            target,
            bend_positive: ik
                .get("bendPositive")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            weight: num(ik, "weight", 1.),
        });
    }
    if !out.is_empty() {
        b.warn(format!(
            "{} IK constraint(s) baked into the animations; the setup pose is shown without IK",
            out.len()
        ));
    }
    out
}

/// DragonBones local transform of every bone at a frame of an animation.
fn locals_at(
    db_bones: &Vec<DbBone>,
    tracks: &HashMap<usize, [Option<Track>; 3]>,
    f: i32,
    duration: i32,
    looping: bool,
) -> Vec<DbTransform> {
    db_bones
        .iter()
        .enumerate()
        .map(|(i, db)| {
            let mut t = db.transform;
            if let Some([tr, rot, sc]) = tracks.get(&i) {
                if let Some(tr) = tr {
                    let v = tr.eval(f as f32, duration, looping);
                    t.x += v[0];
                    t.y += v[1];
                }
                if let Some(rot) = rot {
                    t.rot += rot.eval(f as f32, duration, looping)[0];
                }
                if let Some(sc) = sc {
                    let v = sc.eval(f as f32, duration, looping);
                    t.sx *= v[0];
                    t.sy *= v[1];
                }
            }
            t
        })
        .collect()
}

/// SkelForm local transforms of the baked bones for a DragonBones pose.
fn baked_locals(
    db_bones: &Vec<DbBone>,
    globals: &[Global],
    baked: &[usize],
) -> Vec<(usize, (Vec2, f32, Vec2))> {
    baked
        .iter()
        .map(|&i| {
            let world = skelform_world(&globals[i]);
            let parent = db_bones[i].parent.map(|p| skelform_world(&globals[p]));
            (i, skelform_local(world, parent))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn import_animation(
    a: &Value,
    frame_rate: i32,
    db_bones: &Vec<DbBone>,
    bones: &Vec<Bone>,
    slots: &[SlotInfo],
    slots_raw: &[Value],
    slot_tex: &[Vec<String>],
    world_scale: &dyn Fn(&dyn Fn(usize) -> Vec2) -> Vec<Vec2>,
    poser: &Poser,
    baked: &[usize],
    b: &mut Builder,
) -> Animation {
    type AE = AnimElement;
    let name = string(a, "name").to_string();
    let duration = int(a, "duration", 1).max(1) as i32;
    let looping = int(a, "playTimes", 1) == 0;
    let mut keys: Vec<Keyframe> = vec![];
    if !arr(a, "ffd").is_empty() {
        b.warn(format!(
            "animation '{name}': mesh deform (FFD) timelines aren't supported"
        ));
    }
    if !arr(a, "frame").is_empty() {
        b.warn(format!(
            "animation '{name}': events/actions aren't supported"
        ));
    }

    let db_idx = |n: &str| db_bones.iter().position(|x| x.name == n);
    let bone_by_id = |id: i32| bones.iter().find(|x| x.id == id).unwrap();
    let timelines = arr(a, "bone");

    // scale tracks, needed up front: inheritScale-off bones divide out their parent's
    // (possibly animated) world scale
    let mut scale_tracks: HashMap<usize, Track> = HashMap::new();
    for tl in timelines {
        if let (Some(i), false) = (db_idx(string(tl, "name")), arr(tl, "scaleFrame").is_empty()) {
            let track = Track::parse(arr(tl, "scaleFrame"), |f| {
                vec![num(f, "x", 1.), num(f, "y", 1.)]
            });
            scale_tracks.insert(i, track);
        }
    }

    for tl in timelines {
        let Some(i) = db_idx(string(tl, "name")) else {
            continue;
        };
        let id = b.bone_ids[i];
        let rest = bone_by_id(id).clone();
        let t = db_bones[i].transform;

        if !arr(tl, "frame").is_empty() {
            b.warn(format!("animation '{name}': combined bone frames ('frame') aren't supported; use DragonBones 5.5 export"));
        }

        let frames = arr(tl, "translateFrame");
        if !frames.is_empty() {
            let track = Track::parse(frames, |f| vec![num(f, "x", 0.), num(f, "y", 0.)]);
            let map = |v: &[f32]| vec![rest.pos.x + v[0], rest.pos.y - v[1]];
            keys.extend(track_keys(
                &track,
                id,
                &[AE::PositionX, AE::PositionY],
                duration,
                looping,
                &map,
            ));
        }

        let frames = arr(tl, "rotateFrame");
        if !frames.is_empty() {
            let unwrapped = unwrap_rotations(frames);
            let mut k = 0;
            let track = Track::parse(frames, |_| {
                k += 1;
                vec![unwrapped[k - 1]]
            })
            .wrap_short();
            let map = |v: &[f32]| vec![rest.rot - v[0]];
            keys.extend(track_keys(
                &track,
                id,
                &[AE::Rotation],
                duration,
                looping,
                &map,
            ));
        }

        // scale: SkelForm local = DragonBones world / parent world
        let mut chain_animated = false;
        let mut p = db_bones[i].parent;
        while let Some(pi) = p {
            chain_animated |= scale_tracks.contains_key(&pi);
            p = db_bones[pi].parent;
        }
        let own_rest = Vec2::new(t.sx, t.sy);
        if let Some(track) = scale_tracks.get(&i) {
            if !chain_animated || db_bones[i].inherit_scale {
                let parent_world = if db_bones[i].inherit_scale {
                    Vec2::new(1., 1.)
                } else {
                    db_bones[i]
                        .parent
                        .map(|p| {
                            world_scale(&|j| {
                                Vec2::new(db_bones[j].transform.sx, db_bones[j].transform.sy)
                            })[p]
                        })
                        .unwrap_or(Vec2::new(1., 1.))
                };
                let map = |v: &[f32]| {
                    let s = own_rest * Vec2::new(v[0], v[1]) / parent_world;
                    vec![s.x, s.y]
                };
                keys.extend(track_keys(
                    track,
                    id,
                    &[AE::ScaleX, AE::ScaleY],
                    duration,
                    looping,
                    &map,
                ));
                continue;
            }
        }
        if chain_animated && !db_bones[i].inherit_scale {
            // sample: this bone's local scale changes whenever an ancestor's does
            for f in 0..=duration {
                let own = |j: usize| {
                    let rest = Vec2::new(db_bones[j].transform.sx, db_bones[j].transform.sy);
                    match scale_tracks.get(&j) {
                        Some(tr) => {
                            let v = tr.eval(f as f32, duration, looping);
                            rest * Vec2::new(v[0], v[1])
                        }
                        None => rest,
                    }
                };
                let ws = world_scale(&own);
                let parent = db_bones[i]
                    .parent
                    .map(|p| ws[p])
                    .unwrap_or(Vec2::new(1., 1.));
                let s = ws[i] / parent;
                for (el, v) in [(AE::ScaleX, s.x), (AE::ScaleY, s.y)] {
                    keys.push(Keyframe {
                        frame: f,
                        bone_id: id,
                        element: el,
                        value: v,
                        start_handle: utils::interp_preset(HandlePreset::Linear).0,
                        end_handle: utils::interp_preset(HandlePreset::Linear).1,
                        ..Default::default()
                    });
                }
            }
        }
    }
    // inheritScale-off bones without a scale timeline of their own, under an animated parent
    for i in 0..db_bones.len() {
        if db_bones[i].inherit_scale
            || timelines
                .iter()
                .any(|tl| string(tl, "name") == db_bones[i].name)
        {
            continue;
        }
        let mut animated = false;
        let mut p = db_bones[i].parent;
        while let Some(pi) = p {
            animated |= scale_tracks.contains_key(&pi);
            p = db_bones[pi].parent;
        }
        if !animated {
            continue;
        }
        let id = b.bone_ids[i];
        for f in 0..=duration {
            let own = |j: usize| {
                let rest = Vec2::new(db_bones[j].transform.sx, db_bones[j].transform.sy);
                match scale_tracks.get(&j) {
                    Some(tr) => {
                        let v = tr.eval(f as f32, duration, looping);
                        rest * Vec2::new(v[0], v[1])
                    }
                    None => rest,
                }
            };
            let ws = world_scale(&own);
            let parent = db_bones[i]
                .parent
                .map(|p| ws[p])
                .unwrap_or(Vec2::new(1., 1.));
            let s = ws[i] / parent;
            for (el, v) in [(AE::ScaleX, s.x), (AE::ScaleY, s.y)] {
                keys.push(Keyframe {
                    frame: f,
                    bone_id: id,
                    element: el,
                    value: v,
                    start_handle: utils::interp_preset(HandlePreset::Linear).0,
                    end_handle: utils::interp_preset(HandlePreset::Linear).1,
                    ..Default::default()
                });
            }
        }
    }

    // slots: display (texture) and color
    for tl in arr(a, "slot") {
        let sname = string(tl, "name");
        let carriers: Vec<(i32, Option<usize>)> = slots
            .iter()
            .filter(|s| s.name == sname)
            .map(|s| (s.carrier, s.display))
            .collect();
        if carriers.is_empty() {
            continue;
        }
        let tex = slots_raw
            .iter()
            .position(|s| string(s, "name") == sname)
            .and_then(|z| slot_tex.get(z))
            .cloned()
            .unwrap_or_default();

        let frames = arr(tl, "displayFrame");
        let mut start = 0;
        for f in frames {
            let value = int(f, "value", int(f, "displayIndex", 0));
            let shown = usize::try_from(value).ok();
            for (carrier_id, own) in &carriers {
                // a per-display carrier only shows its own display
                let name = match own {
                    Some(i) if shown == Some(*i) => tex.get(*i).cloned().unwrap_or_default(),
                    Some(_) => String::new(),
                    None => shown.and_then(|i| tex.get(i)).cloned().unwrap_or_default(),
                };
                keys.push(Keyframe {
                    frame: start,
                    bone_id: *carrier_id,
                    element: AE::Texture,
                    value_str: name,
                    start_handle: Vec2::new(999., 999.),
                    end_handle: Vec2::new(999., 999.),
                    ..Default::default()
                });
            }
            start += int(f, "duration", 1).max(0) as i32;
        }

        let frames = arr(tl, "colorFrame");
        if !frames.is_empty() {
            let track = Track::parse(frames, |f| {
                let c = f
                    .get("value")
                    .or(f.get("color"))
                    .cloned()
                    .unwrap_or(Value::Null);
                vec![
                    num(&c, "rM", 100.) / 100.,
                    num(&c, "gM", 100.) / 100.,
                    num(&c, "bM", 100.) / 100.,
                    num(&c, "aM", 100.) / 100.,
                ]
            });
            let map = |v: &[f32]| v.to_vec();
            let els = [AE::TintR, AE::TintG, AE::TintB, AE::TintA];
            for (carrier_id, _) in &carriers {
                keys.extend(track_keys(
                    &track,
                    *carrier_id,
                    &els,
                    duration,
                    looping,
                    &map,
                ));
            }
        }
    }

    // draw order
    if let Some(z) = a.get("zOrder") {
        let count = slots_raw.len();
        let mut start = 0;
        for f in arr(z, "frame") {
            let mut order: Vec<usize> = (0..count).collect();
            let pairs = arr(f, "zOrder");
            if !pairs.is_empty() {
                order = resolve_zorder(pairs, count);
            }
            // order[position] = slot index → zindex of each slot = its position
            for (pos, slot_idx) in order.iter().enumerate() {
                let sname = string(&slots_raw[*slot_idx], "name");
                for s in slots.iter().filter(|s| s.name == sname) {
                    keys.push(Keyframe {
                        frame: start,
                        bone_id: s.carrier,
                        element: AE::Zindex,
                        value: pos as f32,
                        start_handle: Vec2::new(999., 999.),
                        end_handle: Vec2::new(999., 999.),
                        ..Default::default()
                    });
                }
            }
            start += int(f, "duration", 1).max(0) as i32;
        }
    }

    if !arr(a, "ik").is_empty() {
        b.warn(format!("animation '{name}': IK timelines aren't supported"));
    }

    // baked bones: sampled every frame from DragonBones' own pose (IK, inherit flags)
    if !baked.is_empty() {
        let mut tracks: HashMap<usize, [Option<Track>; 3]> = HashMap::new();
        for tl in timelines {
            let Some(i) = db_idx(string(tl, "name")) else {
                continue;
            };
            let tr = Some(arr(tl, "translateFrame"))
                .filter(|f| !f.is_empty())
                .map(|f| Track::parse(f, |x| vec![num(x, "x", 0.), num(x, "y", 0.)]));
            let rot = Some(arr(tl, "rotateFrame"))
                .filter(|f| !f.is_empty())
                .map(|f| {
                    let unwrapped = unwrap_rotations(f);
                    let mut k = 0;
                    Track::parse(f, |_| {
                        k += 1;
                        vec![unwrapped[k - 1]]
                    })
                    .wrap_short()
                });
            let sc = Some(arr(tl, "scaleFrame"))
                .filter(|f| !f.is_empty())
                .map(|f| Track::parse(f, |x| vec![num(x, "x", 1.), num(x, "y", 1.)]));
            tracks.insert(i, [tr, rot, sc]);
        }
        let baked_ids: Vec<i32> = baked.iter().map(|i| b.bone_ids[*i]).collect();
        let transform = [
            AE::PositionX,
            AE::PositionY,
            AE::Rotation,
            AE::ScaleX,
            AE::ScaleY,
        ];
        keys.retain(|k| !(baked_ids.contains(&k.bone_id) && transform.contains(&k.element)));
        let (h0, h1) = utils::interp_preset(HandlePreset::Linear);
        let mut prev_rot: HashMap<usize, f32> = HashMap::new();
        for f in 0..=duration {
            let locals = locals_at(db_bones, &tracks, f, duration, looping);
            let (_, globals) = poser.pose(&locals, true);
            for (i, (pos, rot, scale)) in baked_locals(db_bones, &globals, baked) {
                // keep rotations continuous so SkelForm tweens the short way
                let rot = match prev_rot.get(&i) {
                    Some(p) => p + normalize(rot - p),
                    None => rot,
                };
                prev_rot.insert(i, rot);
                let values = [pos.x, pos.y, rot, scale.x, scale.y];
                for (el, v) in transform.iter().zip(values) {
                    keys.push(Keyframe {
                        frame: f,
                        bone_id: b.bone_ids[i],
                        element: el.clone(),
                        value: v,
                        start_handle: h0,
                        end_handle: h1,
                        ..Default::default()
                    });
                }
            }
        }
    }

    // SkelForm's animation ends at its last key: make sure one sits at `duration`
    if !keys.iter().any(|k| k.frame >= duration) {
        if let Some(root) = bones.first() {
            for (el, v) in [(AE::PositionX, root.pos.x), (AE::PositionY, root.pos.y)] {
                for f in [0, duration] {
                    keys.push(Keyframe {
                        frame: f,
                        bone_id: root.id,
                        element: el.clone(),
                        value: v,
                        start_handle: utils::interp_preset(HandlePreset::Linear).0,
                        end_handle: utils::interp_preset(HandlePreset::Linear).1,
                        ..Default::default()
                    });
                }
            }
        }
    }

    // dedupe same (frame, bone, element), keeping the last
    let mut deduped: Vec<Keyframe> = vec![];
    for k in keys {
        if let Some(i) = deduped
            .iter()
            .position(|d| d.frame == k.frame && d.bone_id == k.bone_id && d.element == k.element)
        {
            deduped[i] = k;
        } else {
            deduped.push(k);
        }
    }
    let mut anim = Animation {
        name,
        fps: frame_rate,
        keyframes: deduped,
        ..Default::default()
    };
    anim.sort_keyframes();
    anim
}

/// Resolve a zOrder frame's (slot index, offset) pairs into the full order, like
/// JSONDataParser::_parseZOrderFrame. Returns order[position] = slot index.
fn resolve_zorder(pairs: &[Value], count: usize) -> Vec<usize> {
    let pairs: Vec<i64> = pairs.iter().filter_map(|x| x.as_i64()).collect();
    let mut z: Vec<i64> = vec![-1; count];
    let mut unchanged: Vec<usize> = vec![];
    let mut original = 0usize;
    for pair in pairs.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        let slot = pair[0] as usize;
        while original < slot && original < count {
            unchanged.push(original);
            original += 1;
        }
        let pos = (original as i64 + pair[1]).clamp(0, count as i64 - 1) as usize;
        z[pos] = original as i64;
        original += 1;
    }
    while original < count {
        unchanged.push(original);
        original += 1;
    }
    let mut out = vec![0usize; count];
    for i in (0..count).rev() {
        out[i] = if z[i] == -1 {
            unchanged.pop().unwrap_or(i)
        } else {
            z[i] as usize
        };
    }
    out
}
