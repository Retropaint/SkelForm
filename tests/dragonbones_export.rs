//! DragonBones exporter tests.
//!
//! Set `DB_OUT_DIR` to also write the exported files plus `<name>_ref.json`, a dump of SkelForm's
//! own world-space pose per frame (in DragonBones' Y-down space) for comparing against a runtime.

use serde_json::{json, Value};
use skelform_lib::dragonbones_export::{self, DbFiles};
use skelform_lib::renderer::{construction, create_tex_rect, is_facing_left};
use skelform_lib::shared::*;
use skelform_lib::utils;

fn load_skf(path: &str) -> Armature {
    let mut shared = Shared::default();
    let ctx = egui::Context::default();
    let file = std::fs::File::open(path).unwrap();
    utils::import(file, &mut shared, None, None, None, Some(&ctx));
    shared.armature
}

fn export(arm: &Armature, name: &str) -> (DbFiles, Value, Value) {
    let files = dragonbones_export::export(arm, name, Vec2::new(2., 2.)).unwrap();
    let ske: Value = serde_json::from_str(&files.ske_json).unwrap();
    let tex: Value = serde_json::from_str(&files.tex_json).unwrap();
    if let Ok(dir) = std::env::var("DB_OUT_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(format!("{}_ske.json", name)), &files.ske_json).unwrap();
        std::fs::write(dir.join(format!("{}_tex.json", name)), &files.tex_json).unwrap();
        std::fs::write(dir.join(format!("{}_tex.png", name)), &files.tex_png).unwrap();
        let reference = reference_dump(arm, &ske);
        let path = dir.join(format!("{}_ref.json", name));
        std::fs::write(path, serde_json::to_string(&reference).unwrap()).unwrap();
    }
    (files, ske, tex)
}

fn is_int(v: &Value) -> bool {
    v.is_i64() || v.is_u64()
}

/// Structural checks: references resolve, and fields DragonBonesCPP reads as ints are ints.
fn validate(ske: &Value, tex: &Value) {
    assert_eq!(ske["version"], "5.5");
    let arm = &ske["armature"][0];
    let bones: Vec<&str> = arm["bone"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();
    let subs: Vec<&str> = tex["SubTexture"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert!(is_int(&ske["frameRate"]) && is_int(&arm["frameRate"]));

    for b in arm["bone"].as_array().unwrap() {
        if let Some(p) = b.get("parent") {
            assert!(bones.contains(&p.as_str().unwrap()), "missing parent {}", p);
        }
    }
    let slots: Vec<&str> = arm["slot"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    for s in arm["slot"].as_array().unwrap() {
        assert!(bones.contains(&s["parent"].as_str().unwrap()));
        assert!(is_int(&s["displayIndex"]));
        if let Some(c) = s.get("color") {
            for k in ["aM", "rM", "gM", "bM"] {
                assert!(is_int(&c[k]));
            }
        }
    }

    for s in arm["skin"][0]["slot"].as_array().unwrap() {
        assert!(slots.contains(&s["name"].as_str().unwrap()));
        for d in s["display"].as_array().unwrap() {
            assert!(
                subs.contains(&d["name"].as_str().unwrap()),
                "display {} not in atlas",
                d["name"]
            );
            if d["type"] == "mesh" {
                let verts = d["vertices"].as_array().unwrap().len();
                assert_eq!(verts, d["uvs"].as_array().unwrap().len());
                let vcount = verts / 2;
                for t in d["triangles"].as_array().unwrap() {
                    assert!(is_int(t) && (t.as_u64().unwrap() as usize) < vcount);
                }
                if let Some(w) = d.get("weights") {
                    let w = w.as_array().unwrap();
                    let mut i = 0;
                    for _ in 0..vcount {
                        let n = w[i].as_u64().unwrap() as usize;
                        i += 1;
                        let mut sum = 0.;
                        for _ in 0..n {
                            assert!(
                                is_int(&w[i]) && (w[i].as_u64().unwrap() as usize) < bones.len()
                            );
                            sum += w[i + 1].as_f64().unwrap();
                            i += 2;
                        }
                        assert!((sum - 1.).abs() < 1e-3, "weights sum to {}", sum);
                    }
                    assert_eq!(i, w.len());
                    assert_eq!(d["bonePose"].as_array().unwrap().len() % 7, 0);
                    assert_eq!(d["slotPose"].as_array().unwrap().len(), 6);
                }
            }
        }
    }

    for anim in arm["animation"].as_array().unwrap() {
        let duration = anim["duration"].as_i64().unwrap();
        assert!(duration >= 1);
        assert!(is_int(&anim["playTimes"]));
        let check_frames = |frames: &Value, tweened: bool| {
            let frames = frames.as_array().unwrap();
            let mut total = 0;
            for (i, f) in frames.iter().enumerate() {
                assert!(is_int(&f["duration"]));
                total += f["duration"].as_i64().unwrap();
                if tweened && i == frames.len() - 1 {
                    assert!(
                        f.get("tweenEasing").is_none() && f.get("curve").is_none(),
                        "last frame tweens"
                    );
                }
            }
            assert_eq!(total, duration);
        };
        for tl in anim["bone"].as_array().unwrap() {
            assert!(bones.contains(&tl["name"].as_str().unwrap()));
            for key in ["translateFrame", "rotateFrame", "scaleFrame"] {
                if let Some(f) = tl.get(key) {
                    check_frames(f, true);
                }
            }
        }
        for tl in anim["slot"].as_array().unwrap() {
            assert!(slots.contains(&tl["name"].as_str().unwrap()));
            if let Some(f) = tl.get("displayFrame") {
                check_frames(f, false);
                for fr in f.as_array().unwrap() {
                    assert!(is_int(&fr["value"]));
                }
            }
            if let Some(f) = tl.get("colorFrame") {
                check_frames(f, true);
            }
        }
        if let Some(z) = anim.get("zOrder") {
            check_frames(&z["frame"], false);
        }
    }
}

// ------------------------------------------------------------------ reference dump

fn tex_size(arm: &Armature, name: &str) -> Option<Vec2> {
    let any_active = arm.styles.iter().any(|s| s.active);
    arm.styles
        .iter()
        .filter(|s| s.active || !any_active)
        .find_map(|s| s.textures.iter().find(|t| t.name == name))
        .map(|t| t.size)
}

/// SkelForm's own pose at a frame, in DragonBones space, keyed by bone index.
fn pose(arm: &Armature, mut local: Vec<Bone>) -> Value {
    for bone in &mut local {
        if let Some(size) = tex_size(arm, &bone.tex) {
            if !bone.verts_edited {
                (bone.vertices, bone.indices) = create_tex_rect(&size);
            }
        }
    }
    let mut world = local.clone();
    construction(&mut world, &local);

    let hidden = |mut id: i32| {
        while let Some(b) = world.iter().find(|b| b.id == id) {
            if b.hidden {
                return true;
            }
            id = b.parent_id;
        }
        false
    };

    let mut order: Vec<usize> = (0..world.len()).collect();
    order.sort_by_key(|i| (world[*i].zindex, *i));

    let mut bones = vec![];
    let mut slots = vec![];
    for (i, b) in world.iter().enumerate() {
        let (sin, cos) = (-b.rot).sin_cos();
        bones.push(json!([
            cos * b.scale.x,
            sin * b.scale.x,
            -sin * b.scale.y,
            cos * b.scale.y,
            b.pos.x,
            -b.pos.y
        ]));

        let mut verts = vec![];
        if let Some(size) = tex_size(arm, &b.tex) {
            if !hidden(b.id) {
                let left = if is_facing_left(b.scale) { -1. } else { 1. };
                let pivot = utils::rotate(&(size * b.pivot_pos), b.rot * left) * b.scale;
                for v in &b.vertices {
                    verts.push(v.pos.x + pivot.x);
                    verts.push(-(v.pos.y + pivot.y));
                }
            }
        }
        let t = &b.tint;
        slots.push(json!({
            "verts": verts,
            "color": [t.a, t.r, t.g, t.b],
            "zRank": order.iter().position(|o| *o == i).unwrap(),
        }));
    }
    json!({ "bones": bones, "slots": slots })
}

fn reference_dump(arm: &Armature, ske: &Value) -> Value {
    let db_rate = ske["frameRate"].as_i64().unwrap() as f32;
    let mut arm = arm.clone();
    let mut anims = serde_json::Map::new();
    let names: Vec<Value> = ske["armature"][0]["animation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].clone())
        .collect();
    for a in 0..arm.animations.len() {
        arm.animations[a].keyframes.retain(|k| k.frame >= 0);
        arm.animations[a].sort_keyframes();
        let last = arm.animations[a]
            .keyframes
            .iter()
            .map(|k| k.frame)
            .max()
            .unwrap_or(0);
        let scale = db_rate / arm.animations[a].fps as f32;
        let mut frames = vec![];
        for f in 0..=last {
            let bones = arm.animate(a, f, None);
            frames
                .push(json!({ "dbFrame": (f as f32 * scale).round(), "pose": pose(&arm, bones) }));
        }
        anims.insert(names[a].as_str().unwrap().to_string(), json!(frames));
    }
    let bone_names: Vec<Value> = ske["armature"][0]["bone"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].clone())
        .collect();
    json!({ "boneNames": bone_names, "setup": pose(&arm, arm.bones.clone()), "animations": anims })
}

// ------------------------------------------------------------------ synthetic rig

fn bone(id: i32, parent_id: i32, name: &str) -> Bone {
    Bone {
        id,
        parent_id,
        name: name.to_string(),
        scale: Vec2::new(1., 1.),
        pivot_scale: Vec2::new(1., 1.),
        tint: TintColor::new(1., 1., 1., 1.),
        ik_target_id: -1,
        ik_family_id: -1,
        physics_id: -1,
        visuals_id: -1,
        ..Default::default()
    }
}

fn key(
    frame: i32,
    bone_id: i32,
    element: AnimElement,
    value: f32,
    preset: HandlePreset,
) -> Keyframe {
    let (start_handle, end_handle) = utils::interp_preset(preset.clone());
    Keyframe {
        frame,
        bone_id,
        element,
        value,
        start_handle,
        end_handle,
        handle_preset: preset,
        ..Default::default()
    }
}

fn texture(arm: &mut Armature, name: &str, w: u32, h: u32) -> Texture {
    let id = arm.tex_data.len() as i32;
    let mut img = image::RgbaImage::new(w, h);
    for p in img.pixels_mut() {
        *p = image::Rgba([255, (id * 90) as u8, 0, 255]);
    }
    arm.tex_data.push(TextureData {
        id,
        image: image::DynamicImage::ImageRgba8(img),
        bind_group: None,
        ui_img: None,
    });
    Texture {
        name: name.to_string(),
        size: Vec2::new(w as f32, h as f32),
        data_id: id,
        ..Default::default()
    }
}

fn synthetic() -> Armature {
    type AE = AnimElement;
    type HP = HandlePreset;
    let mut arm = Armature::default();
    let tex_a = texture(&mut arm, "a", 40, 20);
    let tex_b = texture(&mut arm, "b", 30, 30);
    let tex_m = texture(&mut arm, "m", 64, 32);
    arm.styles.push(Style {
        name: "default".into(),
        active: true,
        textures: vec![tex_a, tex_b, tex_m],
        ..Default::default()
    });

    let mut root = bone(0, -1, "root");
    root.pos = Vec2::new(10., 20.);

    let mut arm_bone = bone(1, 0, "arm");
    arm_bone.pos = Vec2::new(50., 0.);
    arm_bone.rot = 0.5;
    arm_bone.tex = "a".into();
    arm_bone.zindex = 2;
    arm_bone.pivot_pos = Vec2::new(0.25, -0.1);
    arm_bone.tint = TintColor::new(1., 0.5, 1., 1.);

    let mut hand = bone(2, 1, "hand");
    hand.pos = Vec2::new(30., 10.);
    hand.scale = Vec2::new(-1., 1.);
    hand.rot = -0.3;
    hand.tex = "b".into();
    hand.zindex = 3;

    // weighted mesh: 4 corners + center, center and right corners follow "arm"
    let mut cloak = bone(3, 0, "cloak");
    cloak.pos = Vec2::new(-20., -10.);
    cloak.tex = "m".into();
    cloak.zindex = 1;
    cloak.verts_edited = true;
    let corners = [
        (-32., 16., 0., 0.),
        (32., 16., 1., 0.),
        (32., -16., 1., 1.),
        (-32., -16., 0., 1.),
        (0., 0., 0.5, 0.5),
    ];
    for (i, (x, y, u, v)) in corners.iter().enumerate() {
        cloak.vertices.push(Vertex {
            id: i as u32,
            pos: Vec2::new(*x, *y),
            uv: Vec2::new(*u, *v),
            init_pos: Vec2::new(*x, *y),
            color: Color::new(0, 0, 0, 0),
            add_color: Color::new(0, 0, 0, 0),
            tint: TintColor::new(1., 1., 1., 1.),
            offset_rot: 0.,
        });
    }
    cloak.indices = vec![0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4];
    cloak.binds = vec![BoneBind {
        bone_id: 1,
        is_path: false,
        verts: vec![
            BoneBindVert { id: 1, weight: 0.5 },
            BoneBindVert { id: 2, weight: 0.5 },
            BoneBindVert {
                id: 4,
                weight: 0.25,
            },
        ],
    }];

    arm.bones = vec![root, arm_bone, hand, cloak];

    arm.animations.push(Animation {
        name: "wave".into(),
        fps: 30,
        keyframes: vec![
            // full spin on "arm": must not collapse to the shortest path
            key(0, 1, AE::Rotation, 0.5, HP::Linear),
            key(
                30,
                1,
                AE::Rotation,
                0.5 + std::f32::consts::TAU,
                HP::SineInOut,
            ),
            // X keyed with an ease, Y keyed at a different frame: forces sampling
            key(0, 0, AE::PositionX, 10., HP::Linear),
            key(20, 0, AE::PositionX, 60., HP::SineIn),
            key(10, 0, AE::PositionY, 0., HP::Linear),
            // uniform scale on hand, shared frames: translated 1:1
            key(5, 2, AE::ScaleX, -1., HP::Linear),
            key(5, 2, AE::ScaleY, 1., HP::Linear),
            key(25, 2, AE::ScaleX, -2., HP::SineOut),
            key(25, 2, AE::ScaleY, 2., HP::SineOut),
            // texture swap, hide via parent, draw order swap, tint fade
            Keyframe {
                frame: 15,
                bone_id: 1,
                element: AE::Texture,
                value_str: "b".into(),
                ..key(15, 1, AE::Texture, 0., HP::Linear)
            },
            key(27, 0, AE::Hidden, 1., HP::Linear),
            key(10, 3, AE::Zindex, 10., HP::Linear),
            key(0, 1, AE::TintA, 1., HP::Linear),
            key(20, 1, AE::TintA, 0.2, HP::Linear),
            key(22, 2, AE::Rotation, 1., HP::Snap),
        ],
        ..Default::default()
    });
    arm
}

#[test]
fn synthetic_rig() {
    let arm = synthetic();
    let (_, ske, tex) = export(&arm, "synthetic");
    validate(&ske, &tex);

    let a = &ske["armature"][0];
    // slots sorted back to front by zindex
    let slot_names: Vec<&str> = a["slot"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(slot_names, ["cloak", "arm", "hand"]);

    // Y flip + degrees
    let arm_t = &a["bone"][1]["transform"];
    assert!((arm_t["skY"].as_f64().unwrap() + 0.5f64.to_degrees()).abs() < 1e-3);
    assert_eq!(a["bone"][0]["transform"]["y"], json!(-20.0));

    // weighted mesh
    let cloak = &a["skin"][0]["slot"][0]["display"][0];
    assert_eq!(cloak["type"], "mesh");
    assert!(cloak.get("weights").is_some());

    let anim = &a["animation"][0];
    assert_eq!(anim["playTimes"], 0);
    assert_eq!(anim["duration"], 30);
    let tl = |name: &str| {
        anim["bone"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()
            .clone()
    };

    // a 360° turn gets "clockwise" so DragonBones doesn't take the shortest path
    let rot = tl("arm")["rotateFrame"].clone();
    assert!(rot[0].get("clockwise").is_some(), "{}", rot);
    assert!((rot[1]["rotate"].as_f64().unwrap() + 360.).abs() < 1e-2);

    // mismatched X/Y keys are sampled; hand's scale is translated 1:1 with a hold frame at 0
    assert!(tl("root")["translateFrame"].as_array().unwrap().len() > 3);
    let scale = tl("hand")["scaleFrame"].clone();
    assert_eq!(scale.as_array().unwrap().len(), 3);
    assert_eq!(scale[0]["duration"], 5);
    assert!(scale[0].get("tweenEasing").is_none());
    assert!(scale[1].get("curve").is_some());
    // snap rotation key => previous frame holds
    let hand_rot = tl("hand")["rotateFrame"].clone();
    assert!(hand_rot[0].get("tweenEasing").is_none() && hand_rot[0].get("curve").is_none());

    // display: arm swaps texture at 15 and everything hides at 27
    let slot_tl = |name: &str| {
        anim["slot"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()
            .clone()
    };
    let disp: Vec<i64> = slot_tl("arm")["displayFrame"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["value"].as_i64().unwrap())
        .collect();
    assert_eq!(disp, [0, 1, -1]);
    assert!(anim.get("zOrder").is_some());
}

#[test]
fn skellington_sample() {
    let arm = load_skf("./samples/_skellington.skf");
    assert!(
        arm.bones.len() > 0 && arm.tex_data.len() > 0,
        "sample failed to load"
    );
    let (_, ske, tex) = export(&arm, "skellington");
    validate(&ske, &tex);
}

#[test]
fn skellina_sample() {
    let arm = load_skf("./samples/_skellina.skf");
    assert!(
        arm.bones.len() > 0 && arm.tex_data.len() > 0,
        "sample failed to load"
    );
    let (_, ske, tex) = export(&arm, "skellina");
    validate(&ske, &tex);
}

#[test]
fn base_names() {
    assert_eq!(dragonbones_export::base_name("hero_ske.json"), "hero");
    assert_eq!(dragonbones_export::base_name("hero.json"), "hero");
    assert_eq!(dragonbones_export::base_name("hero"), "hero");
}
