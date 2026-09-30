//! Bind Pose skinning: standard linear blend skinning built from ordinary bones.
//!
//! See `docs/BIND_POSE.md`. For a bind-posed mesh, each bone it binds to gets a child
//! "bind helper" placed exactly at the mesh (owner) bone's transform in the setup pose.
//! Vertices stay in the owner's local space, as with classic binds, so every bind agrees
//! on where a vertex is at rest, and each helper moves it by exactly
//! `bone world · inverse(bone bind pose)` when posed.
//!
//! The bind pose always equals the setup pose: `sync_helpers` re-derives the helpers from
//! the setup pose (it's cheap and idempotent, so it can simply run every frame).
//!
//! Helpers are plain bones, so `.skf` files and runtimes need no changes. Only the editor
//! knows about them (`Bone::bind_owner`, saved in editor.json).

use crate::renderer::{construction, is_facing_left};
use crate::shared::*;
use crate::utils;

/// Helper of `bone_id` serving mesh `owner_id`, if any.
pub fn helper_for(armature: &Armature, bone_id: i32, owner_id: i32) -> Option<i32> {
    armature
        .bones
        .iter()
        .find(|b| b.parent_id == bone_id && b.bind_owner == Some(owner_id))
        .map(|b| b.id)
}

/// Whether a mesh bone uses Bind Pose skinning (an explicit per-mesh switch, so it can be
/// turned on before any binds exist).
pub fn is_bind_posed(armature: &Armature, mesh_id: i32) -> bool {
    armature
        .bones
        .iter()
        .find(|b| b.id == mesh_id)
        .map(|b| b.bind_pose)
        .unwrap_or(false)
}

/// Whether a mesh binds to its own helpers (how bind-posed meshes looked before the switch).
fn binds_to_own_helpers(armature: &Armature, mesh: &Bone) -> bool {
    mesh.binds.iter().any(|bind| {
        armature
            .bones
            .iter()
            .any(|b| b.id == bind.bone_id && b.bind_owner == Some(mesh.id))
    })
}

/// Constructed (world) setup pose, including IK.
fn setup_world(armature: &Armature) -> Vec<Bone> {
    let local = armature.bones.clone();
    let mut world = local.clone();
    construction(&mut world, &local);
    world
}

/// World-space vertex positions of a bone as drawn, incl. the pivot offset the renderer adds.
fn drawn_vertices(armature: &Armature, world: &Vec<Bone>, bone_id: i32) -> Vec<Vec2> {
    let bone = world.iter().find(|b| b.id == bone_id).unwrap();
    let size = armature
        .tex_of(bone_id)
        .map(|t| t.size)
        .unwrap_or(Vec2::ZERO);
    let left = if is_facing_left(bone.scale) { -1. } else { 1. };
    let pivot = utils::rotate(&(size * bone.pivot_pos), bone.rot * left) * bone.scale;
    bone.vertices.iter().map(|v| v.pos + pivot).collect()
}

/// Inverse of `renderer::inherit_vert`: the local vertex that `frame` places at `world_pos`.
fn local_vertex(world_pos: Vec2, frame: &Bone, pivot_rot: f32, pivot_scale: Vec2) -> Vec2 {
    utils::rotate(&(world_pos - frame.pos), -(frame.rot + pivot_rot)) / (frame.scale * pivot_scale)
}

/// Local transform that puts a child of `parent` exactly at `target`'s world transform.
/// Inverse of `renderer::inheritance` (docs/BIND_POSE.md §2.1).
fn local_at(parent: &Bone, target: &Bone) -> (Vec2, f32, Vec2) {
    let rot = if is_facing_left(parent.scale) {
        parent.rot - target.rot
    } else {
        target.rot - parent.rot
    };
    let pos = utils::rotate(&(target.pos - parent.pos), -parent.rot) / parent.scale;
    (pos, rot, target.scale / parent.scale)
}

/// Create the helper of `bone_id` for mesh `owner_id` if missing (transform set by `sync_helpers`).
fn ensure_helper(armature: &mut Armature, bone_id: i32, owner_id: i32) -> i32 {
    if let Some(id) = helper_for(armature, bone_id, owner_id) {
        return id;
    }

    let parent_idx = armature.bones.iter().position(|b| b.id == bone_id).unwrap();
    let parent = armature.bones[parent_idx].clone();
    let owner_name = armature
        .bones
        .iter()
        .find(|b| b.id == owner_id)
        .unwrap()
        .name
        .clone();

    // insert after the parent's subtree, keeping children contiguous
    let mut children = vec![];
    crate::armature_window::get_all_children(&armature.bones, &mut children, &parent);
    let idx = parent_idx + children.len() + 1;

    let ids = armature.bones.iter().map(|b| b.id).collect();
    let helper = Bone {
        id: generate_id(ids),
        name: format!("{}__bind__{}", parent.name, owner_name),
        parent_id: bone_id,
        scale: Vec2::new(1., 1.),
        pivot_scale: Vec2::new(1., 1.),
        tint: TintColor::new(1., 1., 1., 1.),
        ik_target_id: -1,
        ik_family_id: -1,
        physics_id: -1,
        visuals_id: -1,
        zindex: -1,
        group_color: Color::new(0, 0, 0, 0),
        bind_owner: Some(owner_id),
        ..Default::default()
    };
    let id = helper.id;
    armature.bones.insert(idx, helper);
    id
}

/// Re-derive every helper from the current setup pose, so each sits exactly at its mesh
/// owner's transform. Returns whether anything changed.
pub fn sync_helpers(armature: &mut Armature) -> bool {
    if !armature.bones.iter().any(|b| b.bind_owner.is_some()) {
        return false;
    }
    let world = setup_world(armature);
    sync_helpers_to(armature, &world)
}

/// `sync_helpers` against a given constructed rest pose (eg: the Pose Mode snapshot).
fn sync_helpers_to(armature: &mut Armature, world: &Vec<Bone>) -> bool {
    let mut changed = false;
    for b in 0..armature.bones.len() {
        let Some(owner_id) = armature.bones[b].bind_owner else {
            continue;
        };
        let parent_id = armature.bones[b].parent_id;
        let parent = world.iter().find(|w| w.id == parent_id);
        let owner = world.iter().find(|w| w.id == owner_id);
        let (Some(parent), Some(owner)) = (parent, owner) else {
            continue;
        };
        let (pos, rot, scale) = local_at(parent, owner);
        let helper = &mut armature.bones[b];
        let close = (helper.pos - pos).mag() < 1e-4
            && (helper.rot - rot).abs() < 1e-6
            && (helper.scale - scale).mag() < 1e-6;
        if !close {
            helper.pos = pos;
            helper.rot = rot;
            helper.scale = scale;
            changed = true;
        }
    }
    changed
}

/// Convert a mesh bone to bind pose skinning at the current setup pose. Vertices don't move.
pub fn set_bind_pose(armature: &mut Armature, mesh_id: i32) -> Result<(), String> {
    let Some(mesh) = armature.bones.iter().find(|b| b.id == mesh_id) else {
        return Err("bone not found".into());
    };
    if mesh.bind_owner.is_some() {
        return Err("bind helpers can't be bind-posed".into());
    }
    if mesh.vertices.is_empty() {
        return Err("bone has no mesh".into());
    }
    if mesh.binds.iter().any(|b| b.is_path) {
        return Err("meshes with path binds can't be bind-posed".into());
    }
    if is_bind_posed(armature, mesh_id) {
        return Ok(());
    }

    let mut bind_bones = vec![];
    for bind in &mesh.binds {
        let valid = armature.bones.iter().any(|b| b.id == bind.bone_id);
        if valid && !bind_bones.contains(&bind.bone_id) {
            bind_bones.push(bind.bone_id);
        }
    }
    for id in bind_bones.iter().chain([&mesh_id]) {
        let bone = armature.bones.iter().find(|b| b.id == *id).unwrap();
        if bone.scale.x == 0. || bone.scale.y == 0. {
            return Err(format!("bone '{}' has zero scale", bone.name));
        }
    }

    // where the vertices are drawn now, before anything changes
    let world = setup_world(armature);
    let rest = drawn_vertices(armature, &world, mesh_id);
    let owner = world.iter().find(|b| b.id == mesh_id).unwrap().clone();

    let helpers: Vec<(i32, i32)> = bind_bones
        .iter()
        .map(|id| (*id, ensure_helper(armature, *id, mesh_id)))
        .collect();

    let mesh = armature.bones.iter_mut().find(|b| b.id == mesh_id).unwrap();
    for bind in &mut mesh.binds {
        if let Some(h) = helpers.iter().find(|h| h.0 == bind.bone_id) {
            bind.bone_id = h.1;
        }
    }

    // every vertex in the owner's frame, with the world-axis pivot offset baked in
    // (the renderer adds it outside of skinning, so it wouldn't follow the blend)
    for (v, pos) in mesh.vertices.iter_mut().zip(&rest) {
        v.pos = local_vertex(*pos, &owner, mesh.pivot_rot, mesh.pivot_scale);
        v.init_pos = v.pos;
    }
    mesh.pivot_pos = Vec2::ZERO;
    mesh.verts_edited = true;
    mesh.bind_pose = true;

    sync_helpers(armature);
    Ok(())
}

/// Convert a bind-posed mesh back to classic binds. Each vertex is stored in the frame of
/// the bone with its largest effective weight, which is exact for single-bone vertices (as
/// with classic binding). Helpers no longer used are removed.
pub fn clear_bind_pose(armature: &mut Armature, mesh_id: i32) {
    if !is_bind_posed(armature, mesh_id) {
        return;
    }
    let world = setup_world(armature);
    let rest = drawn_vertices(armature, &world, mesh_id);
    let mut mesh = armature
        .bones
        .iter()
        .find(|b| b.id == mesh_id)
        .unwrap()
        .clone();

    // retarget binds from helpers to their bones
    for bind in &mut mesh.binds {
        let helper = armature.bones.iter().find(|b| b.id == bind.bone_id);
        if let Some(h) = helper.filter(|h| h.bind_owner == Some(mesh_id)) {
            bind.bone_id = h.parent_id;
        }
    }

    for (i, v) in mesh.vertices.iter_mut().enumerate() {
        // effective weights: the owner starts at 1, each bind lerps
        let mut weights: Vec<(i32, f32)> = vec![(mesh_id, 1.)];
        for bind in &mesh.binds {
            let Some(bv) = bind.verts.iter().find(|bv| bv.id == v.id as i32) else {
                continue;
            };
            for w in weights.iter_mut() {
                w.1 *= 1. - bv.weight;
            }
            weights.push((bind.bone_id, bv.weight));
        }
        let dominant = weights
            .iter()
            .fold(
                (mesh_id, -1.),
                |best, w| if w.1 > best.1 { *w } else { best },
            )
            .0;
        let frame = world.iter().find(|b| b.id == dominant).unwrap();
        v.pos = local_vertex(rest[i], frame, mesh.pivot_rot, mesh.pivot_scale);
        v.init_pos = v.pos;
    }

    let mesh_mut = armature.bones.iter_mut().find(|b| b.id == mesh_id).unwrap();
    mesh_mut.binds = mesh.binds;
    mesh_mut.vertices = mesh.vertices;
    mesh_mut.bind_pose = false;

    remove_unused_helpers(armature);
}

/// Remove helpers whose mesh is gone or no longer binds to them.
pub fn remove_unused_helpers(armature: &mut Armature) {
    let used: Vec<(i32, i32)> = armature
        .bones
        .iter()
        .flat_map(|b| b.binds.iter().map(move |bind| (b.id, bind.bone_id)))
        .collect();
    armature.bones.retain(|b| match b.bind_owner {
        Some(owner) => used.contains(&(owner, b.id)),
        None => true,
    });
}

/// The bone a new bind on `mesh_id` should target when the user picks `bone_id`: the
/// bone's helper for this mesh if the mesh is bind-posed, otherwise the bone itself.
pub fn bind_target(armature: &mut Armature, bone_id: i32, mesh_id: i32) -> i32 {
    if !is_bind_posed(armature, mesh_id) || bone_id == -1 {
        return bone_id;
    }
    let picked = armature.bones.iter().find(|b| b.id == bone_id);
    if picked.map(|b| b.bind_owner.is_some()).unwrap_or(true) {
        return bone_id;
    }
    let helper = ensure_helper(armature, bone_id, mesh_id);
    sync_helpers(armature);
    helper
}

/// Transforms of every (non-helper) bone, taken when entering Pose Mode.
pub fn pose_snapshot(armature: &Armature) -> PoseSnapshot {
    armature
        .bones
        .iter()
        .filter(|b| b.bind_owner.is_none())
        .map(|b| (b.id, b.pos, b.rot, b.scale))
        .collect()
}

/// Put bones back to a snapshot (leaving Pose Mode, or saving the rest pose).
pub fn apply_pose(armature: &mut Armature, snapshot: &PoseSnapshot) {
    for (id, pos, rot, scale) in snapshot {
        if let Some(bone) = armature.bones.iter_mut().find(|b| b.id == *id) {
            bone.pos = *pos;
            bone.rot = *rot;
            bone.scale = *scale;
        }
    }
}

/// The armature with its rest pose: itself, or with the Pose Mode snapshot applied.
pub fn rest_armature(armature: &Armature, rest: Option<&PoseSnapshot>) -> Armature {
    let mut rest_arm = armature.clone();
    if let Some(snapshot) = rest {
        apply_pose(&mut rest_arm, snapshot);
    }
    rest_arm
}

/// `maintain` against the setup pose.
pub fn maintain(armature: &mut Armature) {
    maintain_with_rest(armature, None);
}

/// Keep bind pose data consistent after structural edits (paste, delete, reparent, undo),
/// then re-derive helpers from the rest pose. Cheap when there are no helpers, so it runs
/// every frame. In Pose Mode, `rest` is the snapshot taken on entry: helpers stay at the
/// rest pose while bones are posed, so meshes deform.
pub fn maintain_with_rest(armature: &mut Armature, rest: Option<&PoseSnapshot>) {
    let any = armature
        .bones
        .iter()
        .any(|b| b.bind_owner.is_some() || b.bind_pose);
    if !any {
        return;
    }

    // files from before the explicit switch: a mesh binding to its own helpers is bind-posed
    let legacy: Vec<i32> = armature
        .bones
        .iter()
        .filter(|b| !b.bind_pose && binds_to_own_helpers(armature, b))
        .map(|b| b.id)
        .collect();
    for bone in armature.bones.iter_mut() {
        if legacy.contains(&bone.id) {
            bone.bind_pose = true;
        }
    }

    // every bind of a bind-posed mesh goes through a helper serving that mesh:
    // - a pasted mesh still binds to the original mesh's helpers: give it its own
    // - a bind that reached a real bone some other way gets that bone's helper
    let mut retargets: Vec<(i32, usize, i32)> = vec![];
    for mesh in &armature.bones {
        for (bi, bind) in mesh.binds.iter().enumerate() {
            let Some(target) = armature.bones.iter().find(|b| b.id == bind.bone_id) else {
                continue;
            };
            match target.bind_owner {
                Some(owner) if owner != mesh.id => retargets.push((mesh.id, bi, target.parent_id)),
                None if mesh.bind_pose && !bind.is_path => retargets.push((mesh.id, bi, target.id)),
                _ => {}
            }
        }
    }
    for (mesh_id, bi, bone_id) in retargets {
        let helper = ensure_helper(armature, bone_id, mesh_id);
        let mesh = armature.bones.iter_mut().find(|b| b.id == mesh_id).unwrap();
        mesh.binds[bi].bone_id = helper;
    }

    remove_unused_helpers(armature);
    let world = setup_world(&rest_armature(armature, rest));
    sync_helpers_to(armature, &world);
}

/// Number of vertices of a classic (non bind-posed) mesh that are in more than one weight
/// bind. Classic binding only places a vertex correctly for one bind, so these are
/// misplaced at rest (see docs/BIND_POSE.md §1).
pub fn classic_multi_bind_verts(armature: &Armature, mesh: &Bone) -> usize {
    if mesh.bind_pose || mesh.bind_owner.is_some() {
        return 0;
    }
    let valid =
        |bind: &BoneBind| !bind.is_path && armature.bones.iter().any(|b| b.id == bind.bone_id);
    mesh.vertices
        .iter()
        .filter(|v| {
            let binds = mesh.binds.iter().filter(|bind| valid(bind));
            binds
                .filter(|bind| bind.verts.iter().any(|bv| bv.id == v.id as i32))
                .count()
                > 1
        })
        .count()
}

/// Result of `set_bind_pose_all`: mesh names converted, and skipped with the reason.
#[derive(Default, Debug)]
pub struct BindAllReport {
    pub converted: Vec<String>,
    pub skipped: Vec<(String, String)>,
}

/// Set Bind Pose on every mesh in the armature that can use it. Meshes without binds are
/// left alone (they follow their bone rigidly, so bind pose changes nothing).
pub fn set_bind_pose_all(armature: &mut Armature) -> BindAllReport {
    let mut report = BindAllReport::default();
    let meshes: Vec<(i32, String)> = armature
        .bones
        .iter()
        .filter(|b| b.bind_owner.is_none() && b.verts_edited && !b.vertices.is_empty())
        .filter(|b| b.binds.iter().any(|bind| bind.bone_id != -1))
        .map(|b| (b.id, b.name.clone()))
        .collect();
    for (id, name) in meshes {
        if is_bind_posed(armature, id) {
            report.converted.push(name);
            continue;
        }
        match set_bind_pose(armature, id) {
            Ok(()) => report.converted.push(name),
            Err(err) => report.skipped.push((name, err)),
        }
    }
    report
}
