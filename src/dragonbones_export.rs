//! DragonBones 5.5 JSON exporter.
//!
//! Produces `<base>_ske.json`, `<base>_tex.json` and `<base>_tex.png`.
//! See `docs/DRAGONBONES_EXPORT.md` for the format notes and the SkelForm → DragonBones mapping.
//!
//! Conventions:
//! - SkelForm is Y-up, DragonBones is Y-down: `y` and rotations are negated.
//! - DragonBones reads several fields with `GetInt`/`GetUint`, so those are always emitted as JSON integers.

use crate::renderer::{construction, get_path_normal_angle, is_facing_left};
use crate::shared::*;
use crate::utils;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub struct DbFiles {
    pub ske_json: String,
    pub tex_json: String,
    pub tex_png: Vec<u8>,
}

/// Strip a `_ske.json`/`.json` suffix from a user-picked file name.
pub fn base_name(file_name: &str) -> String {
    let name = file_name.strip_suffix(".json").unwrap_or(file_name);
    let name = name.strip_suffix("_ske").unwrap_or(name);
    name.to_string()
}

pub fn export(armature: &Armature, base: &str, tex_padding: Vec2) -> Result<DbFiles, String> {
    if armature.bones.is_empty() {
        return Err("Armature has no bones".to_string());
    }

    let textures = resolve_textures(armature);
    let (tex_png, tex_json, tex_sizes) = build_atlas(armature, &textures, base, tex_padding)?;

    let ctx = Ctx::new(armature, &tex_sizes);
    let ske = json!({
        "frameRate": ctx.frame_rate,
        "name": base,
        "version": "5.5",
        "compatibleVersion": "5.5",
        "armature": [ctx.armature_json(base)],
    });

    Ok(DbFiles {
        ske_json: serde_json::to_string_pretty(&ske).unwrap(),
        tex_json: serde_json::to_string_pretty(&tex_json).unwrap(),
        tex_png,
    })
}

/// Textures of the active style(s), deduplicated by name (first active style wins, like `Armature::tex_of`).
/// Falls back to all styles if none are active.
fn resolve_textures(armature: &Armature) -> Vec<Texture> {
    let any_active = armature.styles.iter().any(|s| s.active);
    let mut textures: Vec<Texture> = vec![];
    for style in &armature.styles {
        if any_active && !style.active {
            continue;
        }
        for tex in &style.textures {
            if !textures.iter().any(|t| t.name == tex.name) {
                textures.push(tex.clone());
            }
        }
    }
    textures
}

/// Pack textures into a single PNG atlas. Returns (png, tex json, name → size).
fn build_atlas(
    armature: &Armature,
    textures: &Vec<Texture>,
    base: &str,
    tex_padding: Vec2,
) -> Result<(Vec<u8>, Value, HashMap<String, Vec2>), String> {
    // put every texture in one style, so the packer never splits them across atlases
    let mut pack_arm = Armature {
        styles: vec![Style {
            name: "dragonbones".to_string(),
            active: true,
            textures: textures
                .iter()
                .filter(|t| armature.tex_data(t).is_some())
                .cloned()
                .collect(),
            ..Default::default()
        }],
        tex_data: armature.tex_data.clone(),
        ..Default::default()
    };
    let edit_mode = EditMode {
        export_tex_padding: tex_padding,
        export_img_format: ExportImgFormat::PNG,
        ..Default::default()
    };
    let (mut bufs, sizes) = utils::create_tex_sheet(&mut pack_arm, &edit_mode);
    if bufs.len() != 1 {
        return Err(format!(
            "Textures need {} atlases, but DragonBones export supports only one",
            bufs.len()
        ));
    }

    let mut sub_textures = vec![];
    let mut tex_sizes = HashMap::new();
    for tex in &pack_arm.styles[0].textures {
        sub_textures.push(json!({
            "name": tex.name,
            "x": tex.offset.x as i64,
            "y": tex.offset.y as i64,
            "width": tex.size.x as i64,
            "height": tex.size.y as i64,
        }));
        tex_sizes.insert(tex.name.clone(), tex.size);
    }

    let tex_json = json!({
        "name": base,
        "imagePath": format!("{}_tex.png", base),
        "width": sizes[0],
        "height": sizes[0],
        "SubTexture": sub_textures,
    });

    Ok((bufs.remove(0), tex_json, tex_sizes))
}

/// Round for tidier JSON.
fn r(v: f32) -> f64 {
    (v as f64 * 10000.).round() / 10000.
}

fn deg(rad: f32) -> f32 {
    rad.to_degrees()
}

fn pct(v: f32) -> i64 {
    (v * 100.).round().max(0.) as i64
}

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
    const IDENTITY: Mat = Mat {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        tx: 0.,
        ty: 0.,
    };

    /// DragonBones `Transform::toMatrix` (no skew).
    fn from_trs(x: f32, y: f32, rot: f32, sx: f32, sy: f32) -> Mat {
        let (sin, cos) = rot.sin_cos();
        Mat {
            a: cos * sx,
            b: sin * sx,
            c: -sin * sy,
            d: cos * sy,
            tx: x,
            ty: y,
        }
    }

    fn json(&self) -> Vec<f64> {
        vec![
            r(self.a),
            r(self.b),
            r(self.c),
            r(self.d),
            r(self.tx),
            r(self.ty),
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Ease {
    Hold,
    Linear,
    Curve([f32; 4]),
}

impl Ease {
    /// SkelForm stores a segment's easing on its end keyframe.
    fn from_handles(start: Vec2, end: Vec2) -> Ease {
        if start.y == 999. && end.y == 999. {
            return Ease::Hold;
        }
        // handles on the diagonal (eg: the Linear preset, or all-zero handles) are linear
        if start.x == start.y && end.x == end.y {
            return Ease::Linear;
        }
        Ease::Curve([start.x.clamp(0., 1.), start.y, end.x.clamp(0., 1.), end.y])
    }

    fn write(&self, frame: &mut Map<String, Value>) {
        match self {
            Ease::Hold => {}
            Ease::Linear => {
                frame.insert("tweenEasing".into(), json!(0));
            }
            Ease::Curve(c) => {
                frame.insert("curve".into(), json!([r(c[0]), r(c[1]), r(c[2]), r(c[3])]));
            }
        }
    }
}

/// A tweened keyframe in SkelForm frame space. `ease` applies to the segment starting here.
#[derive(Clone, Debug)]
struct TrackFrame {
    frame: i32,
    values: Vec<f32>,
    ease: Ease,
}

struct Ctx<'a> {
    arm: &'a Armature,
    frame_rate: i64,
    names: HashMap<i32, String>,
    /// SkelForm bone id -> DragonBones bone index (same order as `arm.bones`)
    bone_idx: HashMap<i32, usize>,
    /// resolved texture sizes by name (only textures that made it into the atlas)
    tex_sizes: &'a HashMap<String, Vec2>,
    /// setup rotation per bone (IK-solved for IK family bones)
    setup_rot: Vec<f32>,
    /// bones whose rotation is baked per frame because IK drives them
    ik_bones: Vec<i32>,
    slots: Vec<SlotInfo>,
    /// SkelForm's constructed (world) setup pose
    setup_world_bones: Vec<Bone>,
    /// extra root-level bones that carry path-bound mesh vertices
    helpers: Vec<PathHelper>,
    /// bone id -> root-level helper bone that carries its slot (see `init_pivot_helpers`)
    pivot_helpers: Vec<(i32, String)>,
    /// constructed pose of every frame of every animation (dead keyframes removed)
    anim_worlds: Vec<Vec<Vec<Bone>>>,
}

/// Path binds place vertices at `bind bone pos + rotate(vertex * weight, path normal)`.
/// DragonBones can't derive the normal, so a helper bone with that position/rotation is baked.
struct PathHelper {
    mesh_id: i32,
    bind_idx: usize,
    name: String,
    /// setup pose (SkelForm space)
    pos: Vec2,
    angle: f32,
}

struct SkinWeights {
    weights: Vec<Value>,
    bone_pose: Vec<Value>,
    slot_pose: Vec<f64>,
    /// helper-local positions of path-bound vertices (SkelForm axes)
    overrides: Vec<Option<Vec2>>,
}

/// `name`, or `name_2`, `name_3`... if taken.
fn unique(used: &mut Vec<String>, base: String) -> String {
    let mut name = base.clone();
    let mut n = 2;
    while used.contains(&name) {
        name = format!("{}_{}", base, n);
        n += 1;
    }
    used.push(name.clone());
    name
}

/// Keyframes that SkelForm actually plays: dead (-1) ones dropped, sorted by frame.
fn clean_anim(arm: &Armature, a: usize) -> Armature {
    let mut arm = arm.clone();
    arm.animations[a].keyframes.retain(|kf| kf.frame >= 0);
    arm.animations[a].sort_keyframes();
    arm
}

fn last_frame(anim: &Animation) -> i32 {
    anim.keyframes.iter().map(|k| k.frame).max().unwrap_or(0)
}

/// Constructed pose of every frame of an animation.
fn anim_worlds(arm: &Armature, a: usize) -> Vec<Vec<Bone>> {
    let mut arm = clean_anim(arm, a);
    if arm.animations[a].keyframes.is_empty() {
        return vec![];
    }
    (0..=last_frame(&arm.animations[a]))
        .map(|f| world(arm.animate(a, f, None)))
        .collect()
}

/// Run SkelForm's construction (incl. IK) on a local pose.
fn world(local: Vec<Bone>) -> Vec<Bone> {
    let mut world = local.clone();
    construction(&mut world, &local);
    world
}

struct SlotInfo {
    bone_id: i32,
    name: String,
    displays: Vec<String>,
}

impl<'a> Ctx<'a> {
    fn new(arm: &'a Armature, tex_sizes: &'a HashMap<String, Vec2>) -> Ctx<'a> {
        let frame_rate = arm
            .animations
            .iter()
            .map(|a| a.fps)
            .filter(|f| *f > 0)
            .max();

        // unique bone names, since DragonBones looks bones and slots up by name
        let mut names = HashMap::new();
        let mut used: Vec<String> = vec![];
        for bone in &arm.bones {
            let base = if bone.name.is_empty() {
                "bone".to_string()
            } else {
                bone.name.clone()
            };
            let mut name = base.clone();
            let mut n = 2;
            while used.contains(&name) {
                name = format!("{}_{}", base, n);
                n += 1;
            }
            used.push(name.clone());
            names.insert(bone.id, name);
        }

        let bone_idx = arm
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id, i))
            .collect();

        let mut ctx = Ctx {
            arm,
            frame_rate: frame_rate.unwrap_or(24) as i64,
            names,
            bone_idx,
            tex_sizes,
            setup_rot: arm.bones.iter().map(|b| b.rot).collect(),
            ik_bones: vec![],
            slots: vec![],
            setup_world_bones: world(arm.bones.clone()),
            helpers: vec![],
            pivot_helpers: vec![],
            anim_worlds: (0..arm.animations.len())
                .map(|a| anim_worlds(arm, a))
                .collect(),
        };

        ctx.init_ik();
        ctx.init_slots();
        ctx.init_helpers(&mut used);
        ctx.init_pivot_helpers(&mut used);
        ctx
    }

    /// SkelForm offsets a texture by its pivot in world axes (rotate, then scale by world scale)
    /// and applies the pivot rotation after the world scale, while DragonBones applies display
    /// transforms before the bone's. They only disagree when the world scale is non-uniform, so
    /// those slots ride a sampled root-level helper bone instead.
    fn init_pivot_helpers(&mut self, used: &mut Vec<String>) {
        let mut helpers = vec![];
        for slot in &self.slots {
            let bone = self.bone(slot.bone_id).unwrap();
            let weighted = self.skin_weights(bone, Vec2::new(1., 1.)).is_some();
            let has_pivot = bone.pivot_pos != Vec2::ZERO || bone.pivot_rot != 0.;
            if !has_pivot || (bone.verts_edited && weighted) {
                continue;
            }
            let non_uniform = |world: &Vec<Bone>| {
                let b = world.iter().find(|b| b.id == bone.id).unwrap();
                (b.scale.x.abs() - b.scale.y.abs()).abs() > 1e-4
            };
            let needed = non_uniform(&self.setup_world_bones)
                || self.anim_worlds.iter().flatten().any(|w| non_uniform(w));
            if needed {
                helpers.push((
                    bone.id,
                    unique(used, format!("{}__pivot", self.names[&bone.id])),
                ));
            }
        }
        self.pivot_helpers = helpers;
    }

    fn pivot_helper(&self, bone_id: i32) -> Option<&String> {
        self.pivot_helpers
            .iter()
            .find(|h| h.0 == bone_id)
            .map(|h| &h.1)
    }

    /// Pivot helper transform in SkelForm space: (pos incl. pivot offset, rot, scale).
    fn pivot_pose(
        &self,
        world: &Vec<Bone>,
        bone_id: i32,
        prev_size: Vec2,
    ) -> (Vec2, f32, Vec2, Vec2) {
        let b = world.iter().find(|b| b.id == bone_id).unwrap();
        let size = self.tex_sizes.get(&b.tex).copied().unwrap_or(prev_size);
        let left = if is_facing_left(b.scale) { -1. } else { 1. };
        let offset = utils::rotate(&(size * b.pivot_pos), b.rot * left) * b.scale;
        // SkelForm rotates by the pivot rotation after applying the world scale
        (
            b.pos + offset,
            b.rot + b.pivot_rot,
            b.scale * b.pivot_scale,
            size,
        )
    }

    fn init_helpers(&mut self, used: &mut Vec<String>) {
        let mut helpers = vec![];
        for slot in &self.slots {
            let bone = self.bone(slot.bone_id).unwrap();
            if !bone.verts_edited || bone.vertices.is_empty() {
                continue;
            }
            for (bi, bind) in bone.binds.iter().enumerate() {
                if !bind.is_path || !self.path_bind_valid(bone, bi) {
                    continue;
                }
                let Some((pos, angle)) = self.helper_pose(&self.setup_world_bones, bone, bi) else {
                    continue;
                };
                let name = unique(used, format!("{}__path{}", self.names[&bone.id], bi));
                helpers.push(PathHelper {
                    mesh_id: bone.id,
                    bind_idx: bi,
                    name,
                    pos,
                    angle,
                });
            }
        }
        self.helpers = helpers;
    }

    /// `get_path_normal_angle` unwraps the previous/current/next bind bones, so they must exist.
    fn path_bind_valid(&self, bone: &Bone, bi: usize) -> bool {
        let prev = if bi > 0 { bi - 1 } else { bi };
        let next = (bi + 1).min(bone.binds.len() - 1);
        [prev, bi, next]
            .iter()
            .all(|i| self.bone_idx.contains_key(&bone.binds[*i].bone_id))
    }

    /// Helper bone position and rotation for a path bind, given a constructed pose.
    fn helper_pose(&self, world: &Vec<Bone>, mesh: &Bone, bi: usize) -> Option<(Vec2, f32)> {
        let angle = get_path_normal_angle(world, mesh, bi);
        if angle == f32::MAX || angle.is_nan() {
            return None;
        }
        let bind_bone = world.iter().find(|b| b.id == mesh.binds[bi].bone_id)?;
        Some((bind_bone.pos, angle))
    }

    fn helper_idx(&self, mesh_id: i32, bi: usize) -> Option<usize> {
        let h = self
            .helpers
            .iter()
            .position(|h| h.mesh_id == mesh_id && h.bind_idx == bi)?;
        Some(self.arm.bones.len() + h)
    }

    fn bone(&self, id: i32) -> Option<&Bone> {
        self.bone_idx.get(&id).map(|i| &self.arm.bones[*i])
    }

    fn has_tex(&self, name: &str) -> bool {
        self.tex_sizes.contains_key(name)
    }

    /// Find IK families that are actually solved, and use their solved rotations as the setup pose.
    fn init_ik(&mut self) {
        let bones = &self.arm.bones;
        let mut families = vec![];
        for bone in bones {
            let fid = bone.ik_family_id;
            if fid == -1 || bone.ik_disabled || families.contains(&fid) {
                continue;
            }
            if bones.iter().any(|b| b.id == bone.ik_target_id) {
                families.push(fid);
            }
        }
        self.ik_bones = bones
            .iter()
            .filter(|b| families.contains(&b.ik_family_id))
            .map(|b| b.id)
            .collect();
        if self.ik_bones.is_empty() {
            return;
        }

        let rots = self.solved_local_rots(&self.setup_world_bones);
        for (i, bone) in bones.iter().enumerate() {
            if let Some(rot) = rots.get(&bone.id) {
                self.setup_rot[i] = *rot;
            }
        }
    }

    /// Local rotations of IK bones in a constructed (IK-solved) pose.
    fn solved_local_rots(&self, world: &Vec<Bone>) -> HashMap<i32, f32> {
        let mut rots = HashMap::new();
        for id in &self.ik_bones {
            let bone = world.iter().find(|b| b.id == *id).unwrap();
            let parent = world.iter().find(|b| b.id == bone.parent_id);
            let rot = match parent {
                Some(p) if is_facing_left(p.scale) => -(bone.rot - p.rot),
                Some(p) => bone.rot - p.rot,
                None => bone.rot,
            };
            rots.insert(*id, rot);
        }
        rots
    }

    /// One slot per bone that shows a texture at any point, sorted by setup zindex.
    fn init_slots(&mut self) {
        let mut slots = vec![];
        for bone in &self.arm.bones {
            let mut displays: Vec<String> = vec![];
            if self.has_tex(&bone.tex) {
                displays.push(bone.tex.clone());
            }
            for anim in &self.arm.animations {
                for kf in &anim.keyframes {
                    let is_tex = kf.bone_id == bone.id && kf.element == AnimElement::Texture;
                    if is_tex && self.has_tex(&kf.value_str) && !displays.contains(&kf.value_str) {
                        displays.push(kf.value_str.clone());
                    }
                }
            }
            if displays.is_empty() {
                continue;
            }
            slots.push(SlotInfo {
                bone_id: bone.id,
                name: self.names[&bone.id].clone(),
                displays,
            });
        }

        let order = |s: &SlotInfo| {
            (
                self.bone(s.bone_id).unwrap().zindex,
                self.bone_idx[&s.bone_id],
            )
        };
        slots.sort_by_key(|s| order(s));
        self.slots = slots;
    }

    fn armature_json(&self, base: &str) -> Value {
        let mut arm = json!({
            "type": "Armature",
            "frameRate": self.frame_rate,
            "name": base,
            "aabb": { "x": 0, "y": 0, "width": 0, "height": 0 },
            "bone": self.bones_json(),
            "slot": self.slots_json(),
            "skin": [{ "name": "", "slot": self.skin_slots_json() }],
            "animation": self.animations_json(),
        });
        if let Some(first) = arm["animation"].as_array().and_then(|a| a.first()) {
            let name = first["name"].clone();
            arm["defaultActions"] = json!([{ "gotoAndPlay": name }]);
        }
        arm
    }

    fn bones_json(&self) -> Vec<Value> {
        let mut out = vec![];
        for (i, bone) in self.arm.bones.iter().enumerate() {
            let rot = -deg(self.setup_rot[i]);
            let scale = self.chain_scale(bone.id, |b| b.scale);
            let mut b = json!({
                "name": self.names[&bone.id],
                "transform": {
                    "x": r(bone.pos.x),
                    "y": r(-bone.pos.y),
                    "skX": r(rot),
                    "skY": r(rot),
                    "scX": r(scale.x),
                    "scY": r(scale.y),
                },
            });
            // SkelForm multiplies scales per component instead of concatenating matrices, which
            // DragonBones matches by not inheriting scale and baking the scale product per bone.
            if let Some(name) = self.names.get(&bone.parent_id) {
                b["parent"] = json!(name);
                b["inheritScale"] = json!(false);
            }
            out.push(b);
        }
        for h in &self.helpers {
            let rot = -deg(h.angle);
            out.push(json!({
                "name": h.name,
                "transform": { "x": r(h.pos.x), "y": r(-h.pos.y), "skX": r(rot), "skY": r(rot) },
            }));
        }
        for (bone_id, name) in &self.pivot_helpers {
            let (pos, rot, scale, _) =
                self.pivot_pose(&self.setup_world_bones, *bone_id, Vec2::new(0., 0.));
            let rot = -deg(rot);
            out.push(json!({
                "name": name,
                "transform": {
                    "x": r(pos.x),
                    "y": r(-pos.y),
                    "skX": r(rot),
                    "skY": r(rot),
                    "scX": r(scale.x),
                    "scY": r(scale.y),
                },
            }));
        }
        out
    }

    fn hidden_setup(&self, bone_id: i32) -> bool {
        let mut id = bone_id;
        while let Some(bone) = self.bone(id) {
            if bone.hidden {
                return true;
            }
            id = bone.parent_id;
        }
        false
    }

    fn slots_json(&self) -> Vec<Value> {
        let mut out = vec![];
        for slot in &self.slots {
            let bone = self.bone(slot.bone_id).unwrap();
            let mut display_idx = slot
                .displays
                .iter()
                .position(|d| *d == bone.tex)
                .map(|i| i as i64)
                .unwrap_or(-1);
            if self.hidden_setup(bone.id) {
                display_idx = -1;
            }
            let mut s = json!({
                "name": slot.name,
                "parent": self.pivot_helper(bone.id).unwrap_or(&self.names[&bone.id]),
                "displayIndex": display_idx,
            });
            let t = &bone.tint;
            if (t.r, t.g, t.b, t.a) != (1., 1., 1., 1.) {
                s["color"] =
                    json!({ "rM": pct(t.r), "gM": pct(t.g), "bM": pct(t.b), "aM": pct(t.a) });
            }
            out.push(s);
        }
        out
    }

    /// Product of a per-bone scale along the bone's ancestor chain (SkelForm's world scale).
    fn chain_scale(&self, bone_id: i32, scale_of: impl Fn(&Bone) -> Vec2) -> Vec2 {
        let mut scale = Vec2::new(1., 1.);
        let mut id = bone_id;
        while let Some(bone) = self.bone(id) {
            scale *= scale_of(bone);
            id = bone.parent_id;
        }
        scale
    }

    /// World matrix of a bone's setup pose, in DragonBones space.
    fn setup_world(&self, bone_id: i32) -> Mat {
        match self.setup_world_bones.iter().find(|b| b.id == bone_id) {
            Some(b) => Mat::from_trs(b.pos.x, -b.pos.y, -b.rot, b.scale.x, b.scale.y),
            None => Mat::IDENTITY,
        }
    }

    fn skin_slots_json(&self) -> Vec<Value> {
        let mut out = vec![];
        for slot in &self.slots {
            let bone = self.bone(slot.bone_id).unwrap();
            let displays: Vec<Value> = slot
                .displays
                .iter()
                .map(|tex| self.display_json(bone, tex))
                .collect();
            out.push(json!({ "name": slot.name, "display": displays }));
        }
        out
    }

    fn display_json(&self, bone: &Bone, tex: &str) -> Value {
        let size = self.tex_sizes[tex];
        // with a pivot helper, the helper carries the whole pivot transform
        let (pivot, pivot_rot, pivot_scale) = match self.pivot_helper(bone.id) {
            Some(_) => (Vec2::ZERO, 0., Vec2::new(1., 1.)),
            None => (size * bone.pivot_pos, bone.pivot_rot, bone.pivot_scale),
        };

        if !bone.verts_edited || bone.vertices.is_empty() {
            let rot = -deg(pivot_rot);
            return json!({
                "name": tex,
                "type": "image",
                "transform": {
                    "x": r(pivot.x),
                    "y": r(-pivot.y),
                    "skX": r(rot),
                    "skY": r(rot),
                    "scX": r(pivot_scale.x),
                    "scY": r(pivot_scale.y),
                },
            });
        }

        // mesh: bake the pivot into slot-local vertices
        let skin = self.skin_weights(bone, size);
        let mut vertices = vec![];
        let mut uvs = vec![];
        for (i, v) in bone.vertices.iter().enumerate() {
            let p = utils::rotate(&(v.pos * pivot_scale), pivot_rot) + pivot;
            let p = match &skin {
                Some(skin) => skin.overrides[i].unwrap_or(p),
                None => p,
            };
            vertices.push(r(p.x));
            vertices.push(r(-p.y));
            uvs.push(r(v.uv.x));
            uvs.push(r(v.uv.y));
        }
        let triangles: Vec<i64> = bone.indices.iter().map(|i| *i as i64).collect();

        let mut mesh = json!({
            "name": tex,
            "type": "mesh",
            "width": size.x as i64,
            "height": size.y as i64,
            "vertices": vertices,
            "uvs": uvs,
            "triangles": triangles,
        });

        if let Some(skin) = skin {
            mesh["weights"] = json!(skin.weights);
            mesh["bonePose"] = json!(skin.bone_pose);
            mesh["slotPose"] = json!(skin.slot_pose);
        }

        mesh
    }

    /// Convert SkelForm's sequential bind lerps into linear-blend weights, and path binds into
    /// full weight on their helper bone (with the vertex overridden to its helper-local position).
    fn skin_weights(&self, bone: &Bone, size: Vec2) -> Option<SkinWeights> {
        let owner = self.bone_idx[&bone.id];
        let n = bone.vertices.len();
        let mut per_vert: Vec<Vec<(usize, f32)>> = vec![vec![(owner, 1.)]; n];
        let mut overrides: Vec<Option<Vec2>> = vec![None; n];
        let mut any = false;

        // SkelForm adds the pivot offset in world space, rotated by the owner only
        let owner_world = self
            .setup_world_bones
            .iter()
            .find(|b| b.id == bone.id)
            .unwrap();
        let left = if is_facing_left(owner_world.scale) {
            -1.
        } else {
            1.
        };
        let pivot_world =
            utils::rotate(&(size * bone.pivot_pos), owner_world.rot * left) * owner_world.scale;

        for (bi, bind) in bone.binds.iter().enumerate() {
            if bind.is_path {
                let Some(helper) = self.helper_idx(bone.id, bi) else {
                    continue;
                };
                let angle = self.helpers[helper - self.arm.bones.len()].angle;
                for bv in &bind.verts {
                    let Some(vi) = bone.vertices.iter().position(|v| v.id == bv.id as u32) else {
                        continue;
                    };
                    let local =
                        bone.vertices[vi].pos * bv.weight + utils::rotate(&pivot_world, -angle);
                    per_vert[vi] = vec![(helper, 1.)];
                    overrides[vi] = Some(local);
                    any = true;
                }
                continue;
            }
            let Some(&target) = self.bone_idx.get(&bind.bone_id) else {
                continue;
            };
            for bv in &bind.verts {
                let Some(vi) = bone.vertices.iter().position(|v| v.id == bv.id as u32) else {
                    continue;
                };
                // a weight bind after a path bind would need per-bone local positions; skip it
                if overrides[vi].is_some() {
                    continue;
                }
                any = true;
                let w = bv.weight;
                let entries = &mut per_vert[vi];
                for e in entries.iter_mut() {
                    e.1 *= 1. - w;
                }
                match entries.iter_mut().find(|e| e.0 == target) {
                    Some(e) => e.1 += w,
                    None => entries.push((target, w)),
                }
            }
        }

        if !any {
            return None;
        }

        // every bound bone uses the owner's setup matrix, so each bone sees the same local vertex
        let mut pose = self.setup_world(bone.id);
        let det = pose.a * pose.d - pose.b * pose.c;
        if det.abs() < 1e-8 {
            pose = Mat::IDENTITY;
        }

        let mut used: Vec<usize> = vec![];
        let mut weights = vec![];
        for entries in &per_vert {
            let entries: Vec<&(usize, f32)> = entries.iter().filter(|e| e.1 > 1e-6).collect();
            weights.push(json!(entries.len()));
            for (idx, w) in entries {
                weights.push(json!(idx));
                weights.push(json!(r(*w)));
                if !used.contains(idx) {
                    used.push(*idx);
                }
            }
        }
        used.sort();

        let mut bone_pose = vec![];
        for idx in used {
            bone_pose.push(json!(idx));
            for v in pose.json() {
                bone_pose.push(json!(v));
            }
        }

        Some(SkinWeights {
            weights,
            bone_pose,
            slot_pose: pose.json(),
            overrides,
        })
    }

    // ---------------------------------------------------------------- animations

    fn animations_json(&self) -> Vec<Value> {
        let mut out = vec![];
        let mut used_names: Vec<String> = vec![];
        for a in 0..self.arm.animations.len() {
            let mut name = self.arm.animations[a].name.clone();
            if name.is_empty() {
                name = format!("animation_{}", a);
            }
            let base = name.clone();
            let mut n = 2;
            while used_names.contains(&name) {
                name = format!("{}_{}", base, n);
                n += 1;
            }
            used_names.push(name.clone());
            out.push(self.animation_json(a, &name));
        }
        out
    }

    fn animation_json(&self, a: usize, name: &str) -> Value {
        let arm = clean_anim(self.arm, a);
        let duration_sf = last_frame(&arm.animations[a]);

        // IK, path binds and pivot helpers depend on the constructed pose, so they're sampled
        let worlds = &self.anim_worlds[a];
        let ik_rots: Vec<HashMap<i32, f32>> = if self.ik_bones.is_empty() {
            vec![]
        } else {
            worlds.iter().map(|w| self.solved_local_rots(w)).collect()
        };

        let anim = &arm.animations[a];
        let fps = if anim.fps > 0 {
            anim.fps
        } else {
            self.frame_rate as i32
        };
        let scale = self.frame_rate as f32 / fps as f32;
        let duration = ((duration_sf as f32 * scale).round() as i64).max(1);
        let to_db = |f: i32| (f as f32 * scale).round() as i64;

        type AE = AnimElement;
        let mut bone_tls = vec![];
        for (i, bone) in self.arm.bones.iter().enumerate() {
            let mut tl = Map::new();

            let track = build_track(
                &arm,
                a,
                bone.id,
                &[AE::PositionX, AE::PositionY],
                &[bone.pos.x, bone.pos.y],
                duration_sf,
            );
            if let Some(frames) = track {
                let frames = frames_json(&frames, &to_db, duration, |v, f| {
                    f.insert("x".into(), json!(r(v[0] - bone.pos.x)));
                    f.insert("y".into(), json!(r(-(v[1] - bone.pos.y))));
                });
                tl.insert("translateFrame".into(), json!(frames));
            }

            let rot_track = if self.ik_bones.contains(&bone.id) && !ik_rots.is_empty() {
                let frames = ik_rots
                    .iter()
                    .enumerate()
                    .map(|(f, rots)| TrackFrame {
                        frame: f as i32,
                        values: vec![rots[&bone.id]],
                        ease: Ease::Linear,
                    })
                    .collect();
                Some(simplify(frames))
            } else {
                build_track(&arm, a, bone.id, &[AE::Rotation], &[bone.rot], duration_sf)
            };
            if let Some(frames) = rot_track {
                let rest = self.setup_rot[i];
                tl.insert(
                    "rotateFrame".into(),
                    json!(rotate_frames_json(&frames, &to_db, duration, rest)),
                );
            }

            if let Some(frames) = self.scale_track(&arm, a, bone.id, duration_sf) {
                let frames = frames_json(&frames, &to_db, duration, |v, f| {
                    f.insert("x".into(), json!(r(v[0])));
                    f.insert("y".into(), json!(r(v[1])));
                });
                tl.insert("scaleFrame".into(), json!(frames));
            }

            if !tl.is_empty() {
                tl.insert("name".into(), json!(self.names[&bone.id]));
                bone_tls.push(Value::Object(tl));
            }
        }

        if !worlds.is_empty() {
            for h in &self.helpers {
                bone_tls.push(self.helper_timeline(h, worlds, &to_db, duration));
            }
            for (bone_id, name) in &self.pivot_helpers {
                bone_tls.push(self.pivot_timeline(*bone_id, name, worlds, &to_db, duration));
            }
        }

        let mut slot_tls = vec![];
        for slot in &self.slots {
            let bone = self.bone(slot.bone_id).unwrap();
            let mut tl = Map::new();

            if let Some(frames) = self.display_frames(anim, slot, &to_db, duration) {
                tl.insert("displayFrame".into(), json!(frames));
            }

            let t = &bone.tint;
            let track = build_track(
                &arm,
                a,
                bone.id,
                &[AE::TintR, AE::TintG, AE::TintB, AE::TintA],
                &[t.r, t.g, t.b, t.a],
                duration_sf,
            );
            if let Some(frames) = track {
                let frames = frames_json(&frames, &to_db, duration, |v, f| {
                    f.insert("value".into(), json!({ "rM": pct(v[0]), "gM": pct(v[1]), "bM": pct(v[2]), "aM": pct(v[3]) }));
                });
                tl.insert("colorFrame".into(), json!(frames));
            }

            if !tl.is_empty() {
                tl.insert("name".into(), json!(slot.name));
                slot_tls.push(Value::Object(tl));
            }
        }

        let mut out = json!({
            "name": name,
            "duration": duration,
            "playTimes": 0,
            "fadeInTime": 0,
            "bone": bone_tls,
            "slot": slot_tls,
        });
        if let Some(z) = self.zorder_frames(anim, &to_db, duration) {
            out["zOrder"] = json!({ "frame": z });
        }
        out
    }

    /// Sampled position/rotation of a path helper bone.
    fn helper_timeline(
        &self,
        h: &PathHelper,
        worlds: &Vec<Vec<Bone>>,
        to_db: &dyn Fn(i32) -> i64,
        duration: i64,
    ) -> Value {
        let mesh = self.bone(h.mesh_id).unwrap();
        let mut pos_frames = vec![];
        let mut rot_frames = vec![];
        let (mut pos, mut angle) = (h.pos, h.angle);
        for (f, world) in worlds.iter().enumerate() {
            if let Some((p, a)) = self.helper_pose(world, mesh, h.bind_idx) {
                pos = p;
                // keep the angle continuous; only its direction matters
                angle += utils::shortest_angle_delta(angle, a);
            }
            let frame = f as i32;
            let ease = Ease::Linear;
            pos_frames.push(TrackFrame {
                frame,
                values: vec![pos.x, pos.y],
                ease: ease.clone(),
            });
            rot_frames.push(TrackFrame {
                frame,
                values: vec![angle],
                ease,
            });
        }

        let translate = frames_json(&simplify(pos_frames), to_db, duration, |v, f| {
            f.insert("x".into(), json!(r(v[0] - h.pos.x)));
            f.insert("y".into(), json!(r(-(v[1] - h.pos.y))));
        });
        let rotate = rotate_frames_json(&simplify(rot_frames), to_db, duration, h.angle);
        json!({ "name": h.name, "translateFrame": translate, "rotateFrame": rotate })
    }

    /// Sampled transform of a pivot helper bone, relative to its setup transform.
    fn pivot_timeline(
        &self,
        bone_id: i32,
        name: &str,
        worlds: &Vec<Vec<Bone>>,
        to_db: &dyn Fn(i32) -> i64,
        duration: i64,
    ) -> Value {
        let (rest_pos, rest_rot, rest_scale, mut size) =
            self.pivot_pose(&self.setup_world_bones, bone_id, Vec2::ZERO);
        let (mut pos_frames, mut rot_frames, mut scale_frames) = (vec![], vec![], vec![]);
        let mut angle = rest_rot;
        for (f, world) in worlds.iter().enumerate() {
            let (pos, rot, scale, s) = self.pivot_pose(world, bone_id, size);
            size = s;
            angle += utils::shortest_angle_delta(angle, rot);
            let frame = f as i32;
            let ease = Ease::Linear;
            let div = |v: f32, rest: f32| if rest.abs() > 1e-6 { v / rest } else { v };
            let scale = vec![div(scale.x, rest_scale.x), div(scale.y, rest_scale.y)];
            pos_frames.push(TrackFrame {
                frame,
                values: vec![pos.x, pos.y],
                ease: ease.clone(),
            });
            rot_frames.push(TrackFrame {
                frame,
                values: vec![angle],
                ease: ease.clone(),
            });
            scale_frames.push(TrackFrame {
                frame,
                values: scale,
                ease,
            });
        }

        let translate = frames_json(&simplify(pos_frames), to_db, duration, |v, f| {
            f.insert("x".into(), json!(r(v[0] - rest_pos.x)));
            f.insert("y".into(), json!(r(-(v[1] - rest_pos.y))));
        });
        let rotate = rotate_frames_json(&simplify(rot_frames), to_db, duration, rest_rot);
        let scale = frames_json(&simplify(scale_frames), to_db, duration, |v, f| {
            f.insert("x".into(), json!(r(v[0])));
            f.insert("y".into(), json!(r(v[1])));
        });
        json!({ "name": name, "translateFrame": translate, "rotateFrame": rotate, "scaleFrame": scale })
    }

    /// Scale multiplier over time, relative to the setup's accumulated scale (see `bones_json`).
    /// Translated 1:1 if only one bone in the chain has scale keys, otherwise sampled every frame.
    fn scale_track(
        &self,
        arm: &Armature,
        a: usize,
        bone_id: i32,
        duration: i32,
    ) -> Option<Vec<TrackFrame>> {
        type AE = AnimElement;
        let keys = &arm.animations[a].keyframes;
        let is_scale = |k: &Keyframe| k.element == AE::ScaleX || k.element == AE::ScaleY;
        let mut keyed = vec![];
        let mut id = bone_id;
        while let Some(bone) = self.bone(id) {
            if keys.iter().any(|k| k.bone_id == bone.id && is_scale(k)) {
                keyed.push(bone);
            }
            id = bone.parent_id;
        }

        let div = |v: f32, rest: f32| if rest.abs() > 1e-6 { v / rest } else { v };
        if keyed.len() == 1 {
            let b = keyed[0];
            let mut frames = build_track(
                arm,
                a,
                b.id,
                &[AE::ScaleX, AE::ScaleY],
                &[b.scale.x, b.scale.y],
                duration,
            )?;
            for f in &mut frames {
                f.values = vec![div(f.values[0], b.scale.x), div(f.values[1], b.scale.y)];
            }
            return Some(frames);
        }
        if keyed.is_empty() {
            return None;
        }

        let rest = self.chain_scale(bone_id, |b| b.scale);
        let frames = (0..=duration)
            .map(|f| {
                let scale = self.chain_scale(bone_id, |b| {
                    let x = arm.interpolate_keyframes(a, b.id, AE::ScaleX, b.scale.x, f);
                    let y = arm.interpolate_keyframes(a, b.id, AE::ScaleY, b.scale.y, f);
                    Vec2::new(x, y)
                });
                TrackFrame {
                    frame: f,
                    values: vec![div(scale.x, rest.x), div(scale.y, rest.y)],
                    ease: Ease::Linear,
                }
            })
            .collect();
        Some(simplify(frames))
    }

    /// Slot display index over time: texture swaps + hidden (own or inherited).
    fn display_frames(
        &self,
        anim: &Animation,
        slot: &SlotInfo,
        to_db: &dyn Fn(i32) -> i64,
        duration: i64,
    ) -> Option<Vec<Value>> {
        let mut chain = vec![];
        let mut id = slot.bone_id;
        while let Some(b) = self.bone(id) {
            chain.push(b.id);
            id = b.parent_id;
        }

        let mut frames: Vec<i32> = vec![0];
        let mut relevant = false;
        for kf in &anim.keyframes {
            let tex_key = kf.bone_id == slot.bone_id && kf.element == AnimElement::Texture;
            let hide_key = chain.contains(&kf.bone_id) && kf.element == AnimElement::Hidden;
            if tex_key || hide_key {
                relevant = true;
                frames.push(kf.frame);
            }
        }
        if !relevant {
            return None;
        }
        frames.sort();
        frames.dedup();

        let bone = self.bone(slot.bone_id).unwrap();
        let value_at = |f: i32| -> i64 {
            for id in &chain {
                let b = self.bone(*id).unwrap();
                let prev = utils::get_prev_frame(f, &anim.keyframes, *id, &AnimElement::Hidden);
                let hidden = if prev != usize::MAX {
                    anim.keyframes[prev].value != 0.
                } else {
                    b.hidden
                };
                if hidden {
                    return -1;
                }
            }
            let prev = utils::get_prev_frame(f, &anim.keyframes, bone.id, &AnimElement::Texture);
            let tex = if prev != usize::MAX {
                &anim.keyframes[prev].value_str
            } else {
                &bone.tex
            };
            slot.displays
                .iter()
                .position(|d| d == tex)
                .map(|i| i as i64)
                .unwrap_or(-1)
        };

        let steps: Vec<(i64, i64)> = frames.iter().map(|f| (to_db(*f), value_at(*f))).collect();
        Some(step_frames_json(steps, duration, |v, f| {
            f.insert("value".into(), json!(v));
        }))
    }

    /// Draw order over time, from Zindex keys.
    fn zorder_frames(
        &self,
        anim: &Animation,
        to_db: &dyn Fn(i32) -> i64,
        duration: i64,
    ) -> Option<Vec<Value>> {
        let slot_ids: Vec<i32> = self.slots.iter().map(|s| s.bone_id).collect();
        let mut frames: Vec<i32> = vec![0];
        for kf in &anim.keyframes {
            if kf.element == AnimElement::Zindex && slot_ids.contains(&kf.bone_id) {
                frames.push(kf.frame);
            }
        }
        if frames.len() == 1 {
            return None;
        }
        frames.sort();
        frames.dedup();

        // new index of each slot (by setup index) at a frame
        let order_at = |f: i32| -> Vec<i64> {
            let mut order: Vec<usize> = (0..self.slots.len()).collect();
            let z = |s: usize| {
                let bone = self.bone(self.slots[s].bone_id).unwrap();
                let prev = utils::get_prev_frame(f, &anim.keyframes, bone.id, &AnimElement::Zindex);
                let z = if prev != usize::MAX {
                    anim.keyframes[prev].value as i32
                } else {
                    bone.zindex
                };
                (z, self.bone_idx[&bone.id])
            };
            order.sort_by_key(|s| z(*s));
            let mut new_idx = vec![0; order.len()];
            for (pos, s) in order.iter().enumerate() {
                new_idx[*s] = pos as i64;
            }
            new_idx
        };

        let setup: Vec<i64> = (0..self.slots.len() as i64).collect();
        let steps: Vec<(i64, Vec<i64>)> =
            frames.iter().map(|f| (to_db(*f), order_at(*f))).collect();
        if steps.iter().all(|s| s.1 == setup) {
            return None;
        }
        Some(step_frames_json(steps, duration, |order, f| {
            if *order == setup {
                return;
            }
            let mut pairs = vec![];
            for (i, new) in order.iter().enumerate() {
                pairs.push(i as i64);
                pairs.push(new - i as i64);
            }
            f.insert("zOrder".into(), json!(pairs));
        }))
    }
}

/// Build a tweened track for a group of channels that share one DragonBones frame (eg: X and Y).
///
/// Keyframes are translated 1:1 if all keyed channels share frames and easings; otherwise the
/// group is sampled every frame with linear tweens.
fn build_track(
    arm: &Armature,
    anim_idx: usize,
    bone_id: i32,
    elements: &[AnimElement],
    rests: &[f32],
    duration: i32,
) -> Option<Vec<TrackFrame>> {
    let anim = &arm.animations[anim_idx];
    let chans: Vec<Vec<&Keyframe>> = elements
        .iter()
        .map(|el| {
            anim.keyframes
                .iter()
                .filter(|k| k.bone_id == bone_id && k.element == *el)
                .collect()
        })
        .collect();
    let keyed: Vec<&Vec<&Keyframe>> = chans.iter().filter(|c| !c.is_empty()).collect();
    if keyed.is_empty() {
        return None;
    }

    let same = |a: &&Keyframe, b: &&Keyframe| {
        a.frame == b.frame && a.start_handle == b.start_handle && a.end_handle == b.end_handle
    };
    let no_dupes = keyed
        .iter()
        .all(|c| c.windows(2).all(|w| w[0].frame != w[1].frame));
    let compatible = no_dupes
        && keyed.iter().all(|c| {
            c.len() == keyed[0].len() && c.iter().zip(keyed[0].iter()).all(|(a, b)| same(a, b))
        });

    let mut frames: Vec<TrackFrame> = vec![];
    if compatible {
        let keys = keyed[0];
        for (k, key) in keys.iter().enumerate() {
            let values = chans
                .iter()
                .zip(rests)
                .map(|(c, rest)| if c.is_empty() { *rest } else { c[k].value })
                .collect();
            let ease = match keys.get(k + 1) {
                Some(next) => Ease::from_handles(next.start_handle, next.end_handle),
                None => Ease::Hold,
            };
            frames.push(TrackFrame {
                frame: key.frame,
                values,
                ease,
            });
        }
        // SkelForm holds the first key's value before it
        if frames[0].frame > 0 {
            let mut first = frames[0].clone();
            first.frame = 0;
            first.ease = Ease::Hold;
            frames.insert(0, first);
        }
    } else {
        for f in 0..=duration {
            let values = elements
                .iter()
                .zip(rests)
                .map(|(el, rest)| {
                    arm.interpolate_keyframes(anim_idx, bone_id, el.clone(), *rest, f)
                })
                .collect();
            frames.push(TrackFrame {
                frame: f,
                values,
                ease: Ease::Linear,
            });
        }
        frames = simplify(frames);
    }

    Some(frames)
}

/// Drop sampled frames that sit between two identical values (linear tweens make them redundant).
fn simplify(frames: Vec<TrackFrame>) -> Vec<TrackFrame> {
    let mut out: Vec<TrackFrame> = vec![];
    for i in 0..frames.len() {
        let redundant = i > 0
            && i + 1 < frames.len()
            && frames[i].values == frames[i - 1].values
            && frames[i].values == frames[i + 1].values;
        if !redundant {
            out.push(frames[i].clone());
        }
    }
    out
}

/// Convert frame positions to DragonBones frames with `duration`s.
/// Frames mapping onto the same DragonBones frame keep the latest one. The last frame never tweens.
fn frames_json(
    frames: &[TrackFrame],
    to_db: &dyn Fn(i32) -> i64,
    duration: i64,
    mut write: impl FnMut(&[f32], &mut Map<String, Value>),
) -> Vec<Value> {
    let placed = place(frames, to_db);
    let mut out = vec![];
    for (i, (pos, frame)) in placed.iter().enumerate() {
        let mut f = Map::new();
        let next = placed.get(i + 1).map(|p| p.0);
        f.insert(
            "duration".into(),
            json!(next.unwrap_or(duration).saturating_sub(*pos).max(0)),
        );
        if next.is_some() {
            frame.ease.write(&mut f);
        }
        write(&frame.values, &mut f);
        out.push(Value::Object(f));
    }
    out
}

/// Like `frames_json`, but for rotation: relative to setup, degrees, Y-flipped, and with `clockwise`
/// on segments that turn more than 180° (DragonBones otherwise takes the shortest path).
fn rotate_frames_json(
    frames: &[TrackFrame],
    to_db: &dyn Fn(i32) -> i64,
    duration: i64,
    rest: f32,
) -> Vec<Value> {
    let placed = place(frames, to_db);
    let degs: Vec<f32> = placed
        .iter()
        .map(|(_, f)| -deg(f.values[0] - rest))
        .collect();
    let mut out = vec![];
    for (i, (pos, frame)) in placed.iter().enumerate() {
        let mut f = Map::new();
        let next = placed.get(i + 1).map(|p| p.0);
        f.insert(
            "duration".into(),
            json!(next.unwrap_or(duration).saturating_sub(*pos).max(0)),
        );
        if next.is_some() {
            frame.ease.write(&mut f);
            let delta = degs[i + 1] - degs[i];
            if delta.abs() > 180. {
                f.insert("clockwise".into(), json!(if delta > 0. { 1 } else { -1 }));
            }
        }
        f.insert("rotate".into(), json!(r(degs[i])));
        out.push(Value::Object(f));
    }
    out
}

fn place<'a>(frames: &'a [TrackFrame], to_db: &dyn Fn(i32) -> i64) -> Vec<(i64, &'a TrackFrame)> {
    let mut placed: Vec<(i64, &TrackFrame)> = vec![];
    for frame in frames {
        let pos = to_db(frame.frame);
        if let Some(last) = placed.last_mut() {
            if last.0 == pos {
                *last = (pos, frame);
                continue;
            }
        }
        placed.push((pos, frame));
    }
    placed
}

/// Non-tweened frames from (position, value) steps, merging repeated values.
fn step_frames_json<T: PartialEq>(
    steps: Vec<(i64, T)>,
    duration: i64,
    mut write: impl FnMut(&T, &mut Map<String, Value>),
) -> Vec<Value> {
    let mut merged: Vec<(i64, T)> = vec![];
    for (pos, v) in steps {
        if let Some(last) = merged.last_mut() {
            if last.0 == pos {
                *last = (pos, v);
                continue;
            }
            if last.1 == v {
                continue;
            }
        }
        merged.push((pos, v));
    }

    let mut out = vec![];
    for (i, (pos, v)) in merged.iter().enumerate() {
        let next = merged.get(i + 1).map(|p| p.0).unwrap_or(duration);
        let mut f = Map::new();
        f.insert("duration".into(), json!((next - pos).max(0)));
        write(v, &mut f);
        out.push(Value::Object(f));
    }
    out
}
