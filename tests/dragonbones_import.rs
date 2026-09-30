//! DragonBones importer checked against the reference runtime.
//!
//! Opt-in (`cargo test --test dragonbones_import -- --ignored --nocapture`): needs
//! `DB_SAMPLES` = a folder searched for `*_ske.json` (eg: DragonBonesCPP's
//! `Cocos2DX_3.x/Demos/Resources`) and the harness built at
//! `tools/dragonbones_verify/build/dbharness.exe` (see docs/DRAGONBONES_VERIFY.md).
//!
//! Each rig is imported into SkelForm; the harness plays the *original* files in
//! DragonBonesCPP; bone positions and slot vertices are compared by name, at setup and on
//! every frame of every animation.

use serde_json::Value;
use skelform_lib::dragonbones_import;
use skelform_lib::renderer::{construction, create_tex_rect, is_facing_left};
use skelform_lib::shared::*;
use skelform_lib::utils;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn find_rigs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            find_rigs(&p, out);
        } else if p.to_string_lossy().ends_with("_ske.json") {
            out.push(p);
        }
    }
}

fn tex_size(arm: &Armature, name: &str) -> Option<Vec2> {
    arm.styles
        .iter()
        .filter(|s| s.active)
        .find_map(|s| s.textures.iter().find(|t| t.name == name))
        .map(|t| t.size)
}

/// SkelForm's world bone positions (by name) and drawn vertices (by bone id), DragonBones axes.
fn render(
    arm: &Armature,
    mut local: Vec<Bone>,
) -> (HashMap<String, (Vec2, f32)>, HashMap<i32, Vec<Vec2>>) {
    for bone in &mut local {
        if let Some(size) = tex_size(arm, &bone.tex) {
            if !bone.verts_edited {
                (bone.vertices, bone.indices) = create_tex_rect(&size);
            }
        }
    }
    let mut world = local.clone();
    construction(&mut world, &local);
    let mut bones = HashMap::new();
    let mut verts = HashMap::new();
    for b in &world {
        bones.insert(b.name.clone(), (Vec2::new(b.pos.x, -b.pos.y), -b.rot));
        if let Some(size) = tex_size(arm, &b.tex) {
            let left = if is_facing_left(b.scale) { -1. } else { 1. };
            let pivot = utils::rotate(&(size * b.pivot_pos), b.rot * left) * b.scale;
            let v: Vec<Vec2> = b
                .vertices
                .iter()
                .map(|v| v.pos + pivot)
                .map(|p| Vec2::new(p.x, -p.y))
                .collect();
            verts.insert(b.id, v);
        }
    }
    (bones, verts)
}

struct Errors {
    rot: f32,
    worst_rot: String,
    bone: f32,
    vert: f32,
    worst_bone: String,
    worst_vert: String,
    visibility: usize,
}

/// Distance from `p` to a convex quad (0 inside).
fn dist_to_quad(p: Vec2, quad: &[Vec2]) -> f32 {
    if quad.len() < 3 {
        return f32::MAX;
    }
    // order corners around their centroid, then test each edge
    let c = quad.iter().fold(Vec2::ZERO, |a, b| a + *b) / quad.len() as f32;
    let mut q = quad.to_vec();
    q.sort_by(|a, b| {
        (a.y - c.y)
            .atan2(a.x - c.x)
            .total_cmp(&(b.y - c.y).atan2(b.x - c.x))
    });
    let mut inside = true;
    let mut best = f32::MAX;
    for i in 0..q.len() {
        let (a, b) = (q[i], q[(i + 1) % q.len()]);
        let e = b - a;
        let t = (((p - a).x * e.x + (p - a).y * e.y) / (e.x * e.x + e.y * e.y)).clamp(0., 1.);
        best = best.min((a + e * t - p).mag());
        if e.x * (p - a).y - e.y * (p - a).x < 0. {
            inside = false;
        }
    }
    if inside {
        0.
    } else {
        best
    }
}

fn compare(
    frame: &Value,
    ours: &(HashMap<String, (Vec2, f32)>, HashMap<i32, Vec<Vec2>>),
    slot_bones: &[(String, i32)],
    trimmed: &[i32],
    tag: &str,
    e: &mut Errors,
) {
    if let Some(bones) = frame["bones"].as_object() {
        for (name, m) in bones {
            let Some((p, rot)) = ours.0.get(name) else {
                continue;
            };
            let f = |i: usize| m[i].as_f64().unwrap() as f32;
            let db = Vec2::new(f(4), f(5));
            let d = (*p - db).mag();
            // rotation of the bone's x axis (DragonBones matrix), in degrees
            let db_rot = f(1).atan2(f(0));
            let mut dr = (rot - db_rot).rem_euclid(std::f32::consts::TAU);
            if dr > std::f32::consts::PI {
                dr = std::f32::consts::TAU - dr;
            }
            let dr = dr.to_degrees();
            if dr > e.rot {
                e.rot = dr;
                e.worst_rot = format!("{tag} {name}");
            }
            if d > e.bone {
                e.bone = d;
                e.worst_bone = format!("{tag} {name}");
            }
        }
    }
    if let Some(slots) = frame["slots"].as_object() {
        for (name, s) in slots {
            // a slot may be split into one bone per display: use the visible one
            let ids: Vec<i32> = slot_bones
                .iter()
                .filter(|x| &x.0 == name)
                .map(|x| x.1)
                .collect();
            if ids.is_empty() {
                continue;
            }
            let id = ids
                .iter()
                .find(|i| ours.1.get(i).map(|v| !v.is_empty()).unwrap_or(false))
                .unwrap_or(&ids[0]);
            let db: Vec<Vec2> = s["verts"]
                .as_array()
                .unwrap()
                .chunks(2)
                .map(|c| Vec2::new(c[0].as_f64().unwrap() as f32, c[1].as_f64().unwrap() as f32))
                .collect();
            let mine = ours.1.get(id).cloned().unwrap_or_default();
            if db.is_empty() != mine.is_empty() {
                e.visibility += 1;
                continue;
            }
            // trimmed textures are imported padded back to their full frame: DragonBones' corners
            // (of the trimmed region) must lie inside ours
            let padded = trimmed.contains(id);
            for p in &db {
                let d = if padded {
                    dist_to_quad(*p, &mine)
                } else {
                    mine.iter()
                        .map(|q| (*p - *q).mag())
                        .fold(f32::MAX, f32::min)
                };
                if d > e.vert {
                    e.vert = d;
                    e.worst_vert = format!("{tag} {name}");
                }
            }
        }
    }
}

#[test]
#[ignore]
fn import_dragonbones_samples() {
    let Ok(dir) = std::env::var("DB_SAMPLES") else {
        println!("set DB_SAMPLES to a folder of DragonBones rigs");
        return;
    };
    let harness = Path::new("tools/dragonbones_verify/build/dbharness.exe");
    assert!(
        harness.exists(),
        "build the harness first (tools/dragonbones_verify/build.bat)"
    );
    let out_dir = std::env::temp_dir().join("skelform_db_import");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut rigs = vec![];
    find_rigs(Path::new(&dir), &mut rigs);
    rigs.sort();
    for ske in rigs {
        let name = ske
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace("_ske.json", "");
        let (json, atlases) = match dragonbones_import::read_files(&ske) {
            Ok(x) => x,
            Err(err) => {
                println!("{name:>24}: can't read ({err})");
                continue;
            }
        };
        let imported = match dragonbones_import::import(&json, &atlases) {
            Ok(x) => x,
            Err(err) => {
                println!("{name:>24}: import failed: {err}");
                continue;
            }
        };

        // the original files through DragonBonesCPP
        let tex = ske.with_file_name(format!("{name}_tex.json"));
        let dump = out_dir.join(format!("{name}_db.json"));
        let run = std::process::Command::new(harness)
            .arg(&ske)
            .arg(&tex)
            .arg(&dump)
            .output()
            .unwrap();
        if !run.status.success() {
            println!("{name:>24}: harness failed");
            continue;
        }
        let db: Value = serde_json::from_str(&std::fs::read_to_string(&dump).unwrap()).unwrap();

        let arm = imported.armature;
        let mut trimmed_tex: Vec<String> = vec![];
        for page in &atlases {
            let t: Value = serde_json::from_str(&page.json).unwrap();
            for st in t["SubTexture"].as_array().unwrap() {
                if st.get("frameWidth").is_some() {
                    trimmed_tex.push(st["name"].as_str().unwrap().to_string());
                }
            }
        }
        // image bones that show a trimmed texture at some point (setup or animation keys)
        let trimmed: Vec<i32> = arm
            .bones
            .iter()
            .filter(|b| !b.verts_edited)
            .filter(|b| {
                trimmed_tex.contains(&b.tex)
                    || arm.animations.iter().flat_map(|a| &a.keyframes).any(|k| {
                        k.bone_id == b.id
                            && k.element == AnimElement::Texture
                            && trimmed_tex.contains(&k.value_str)
                    })
            })
            .map(|b| b.id)
            .collect();
        let mut e = Errors {
            rot: 0.,
            worst_rot: String::new(),
            bone: 0.,
            vert: 0.,
            worst_bone: String::new(),
            worst_vert: String::new(),
            visibility: 0,
        };
        // the imported setup pose is shown without IK (DragonBones applies it), so skip it
        let has_ik = serde_json::from_str::<Value>(&json)
            .map(|v| {
                v["armature"][0]["ik"]
                    .as_array()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if !has_ik {
            compare(
                &db["setup"],
                &render(&arm, arm.bones.clone()),
                &imported.slot_bones,
                &trimmed,
                "setup",
                &mut e,
            );
        }
        let mut frames = 0;
        if let Some(anims) = db["animations"].as_object() {
            for (anim_name, a) in anims {
                let Some(ai) = arm.animations.iter().position(|x| &x.name == anim_name) else {
                    continue;
                };
                // the last frame (= duration) is the loop point: never displayed while looping,
                // and the runtimes disagree on which side of it they show
                let all = a["frames"].as_array().unwrap();
                for (f, frame) in all
                    .iter()
                    .enumerate()
                    .take(all.len().saturating_sub(1).max(1))
                {
                    let ours = render(&arm, arm.clone().animate(ai, f as i32, None));
                    compare(
                        frame,
                        &ours,
                        &imported.slot_bones,
                        &trimmed,
                        &format!("{anim_name}@{f}"),
                        &mut e,
                    );
                    frames += 1;
                }
            }
        }
        println!(
            "{name:>22}: {:>3} bones {:>4} frames{} | rot {:6.2}° ({}) | bone {:7.3} ({}) | verts {:7.3} px ({}) | vis {} | warn {}",
            arm.bones.len(),
            frames,
            if has_ik { " ik" } else { "   " },
            e.rot,
            e.worst_rot,
            e.bone,
            e.worst_bone,
            e.vert,
            e.worst_vert,
            e.visibility,
            imported.warnings.len(),
        );
        for w in imported.warnings.iter().take(4) {
            println!("{:>24}    ! {w}", "");
        }
    }
}

/// Every import warning per rig (`DB_SAMPLES` as above).
#[test]
#[ignore]
fn import_dragonbones_warnings() {
    let Ok(dir) = std::env::var("DB_SAMPLES") else { return };
    let mut rigs = vec![];
    find_rigs(Path::new(&dir), &mut rigs);
    rigs.sort();
    for ske in rigs {
        let name = ske.file_name().unwrap().to_string_lossy().replace("_ske.json", "");
        match dragonbones_import::read_files(&ske).and_then(|(j, a)| dragonbones_import::import(&j, &a)) {
            Ok(imp) => {
                println!("== {name}: {} warnings", imp.warnings.len());
                for w in &imp.warnings {
                    println!("   {w}");
                }
            }
            Err(e) => println!("== {name}: FAILED {e}"),
        }
    }
}

/// SkelForm's own rig warnings (the ⚠ list) on each imported rig (`DB_SAMPLES` as above).
#[test]
#[ignore]
fn import_dragonbones_editor_warnings() {
    let Ok(dir) = std::env::var("DB_SAMPLES") else { return };
    let mut rigs = vec![];
    find_rigs(Path::new(&dir), &mut rigs);
    rigs.sort();
    for ske in rigs {
        let name = ske.file_name().unwrap().to_string_lossy().replace("_ske.json", "");
        let Ok(imp) = dragonbones_import::read_files(&ske).and_then(|(j, a)| dragonbones_import::import(&j, &a)) else { continue };
        let warnings = skelform_lib::warnings::check_warnings(&imp.armature);
        let mut counts: HashMap<String, (usize, usize)> = HashMap::new();
        for w in &warnings {
            const NAMES: [&str; 11] = ["SameZIndex", "NoIkTarget", "OnlyIk", "UnboundBind", "NoVertsInBind", "OnlyPath", "NoWeights", "BoneOutOfFamily", "EmptyStyles", "UnusedTextures", "ClassicMultiBind"];
            let kind = NAMES.get(w.warn_type.clone() as usize).unwrap_or(&"?");
            let e = counts.entry(kind.to_string()).or_default();
            e.0 += 1;
            e.1 += w.ids.len().max(w.str_values.len());
        }
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort();
        println!("{name:>22}: {:?}", counts);
    }
}
