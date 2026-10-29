use serde_json::{json, Value};

use crate::{InverseKinematics, Physics, Root, Visuals};

macro_rules! get_array {
    ($src:expr, $name:expr) => {
        $src.get_mut($name).unwrap().as_array_mut().unwrap()
    };
}

// run backwards compatibility checks for this JSON data
pub fn proceed(mut raw: serde_json::Value) -> (Root, String) {
    let mut err = "".to_string();

    macro_rules! from_ver {
        ($ver:expr, $func:ident) => {
            if raw.get("version").unwrap().as_str().unwrap().contains($ver) {
                $func(&mut raw);
            }
        };
    }

    from_ver!("0.2", v0d2_to_v0d3);
    from_ver!("0.3", v0d3_to_v0d4);
    from_ver!("0.4", v0d4_to_v0d4d1);
    from_ver!("0.4.1", v0d4d1_to_v0d5);
    from_ver!("0.5", v0d5_to_v0d6);
    from_ver!("0.6", v0d6_7_8_to_v0d9);
    from_ver!("0.7", v0d6_7_8_to_v0d9);
    from_ver!("0.8", v0d6_7_8_to_v0d9);

    let json_string = serde_json::to_string(&raw).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_string);
    let result: Result<Root, _> = serde_path_to_error::deserialize(&mut deserializer);
    if let Err(invalid) = result {
        err = format!("from v{}: \n\n{}", "test", &invalid.to_string());
    }

    if err == "" {
        // return new JSON version
        return (serde_json::from_value(raw).unwrap(), "".to_string());
    } else {
        // return JSON parsing error
        return (Root::default(), err);
    }
}

pub fn v0d2_to_v0d3(raw: &mut serde_json::Value) {
    raw["version"] = "v0.3.0".into();
}

pub fn v0d3_to_v0d4(raw: &mut serde_json::Value) {
    for bone in get_array!(raw, "bones") {
        if let Some(str) = bone.get("ik_constraint_str") {
            bone["ik_constraint"] = str.clone();
        }
        if let Some(str) = bone.get("ik_mode_str") {
            bone["ik_mode"] = str.clone();
        }
    }
    for anim in get_array!(raw, "animations") {
        for keyframe in get_array!(anim, "keyframes") {
            keyframe["element"] = keyframe["element_str"].clone();
        }
    }
    raw["version"] = "v0.4.0".into();
}

pub fn v0d4_to_v0d4d1(raw: &mut serde_json::Value) {
    if raw.get_mut("animations") != None {
        for anim in get_array!(raw, "animations") {
            if anim.get_mut("keyframes") == None {
                continue;
            }
            for keyframe in get_array!(anim, "keyframes") {
                if keyframe["frame"] == -1 {
                    keyframe["frame"] = serde_json::to_value(0).unwrap();
                }
            }
        }
    }

    raw["version"] = "v0.4.1".into();
}
pub fn v0d4d1_to_v0d5(raw: &mut serde_json::Value) {
    raw["version"] = "v0.5.0".into();
}

pub fn v0d5_to_v0d6(raw: &mut serde_json::Value) {
    // initiate Root
    let json_string = serde_json::to_string(&raw).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_string);
    let mut root: Result<Root, _> = serde_path_to_error::deserialize(&mut deserializer);

    for b in 0..raw.get("bones").unwrap().as_array().unwrap().len() {
        let bone = &raw.get("bones").unwrap().as_array().unwrap()[b];

        let json_string = serde_json::to_string(&bone).unwrap();

        // extract visuals fields from this bone
        let mut deserializer = serde_json::Deserializer::from_str(&json_string);
        let visuals: Result<Visuals, _> = serde_path_to_error::deserialize(&mut deserializer);
        root.as_mut().unwrap().visuals.push(visuals.unwrap());
        let len = root.as_ref().unwrap().visuals.len();
        root.as_mut().unwrap().bones[b].visuals_id = len as i32 - 1;

        // extract IK fields from this bone
        let mut deserializer = serde_json::Deserializer::from_str(&json_string);
        let mut ik_json: Result<Value, _> = serde_path_to_error::deserialize(&mut deserializer);
        if ik_json.as_ref().unwrap().get("ik_bone_ids") != None {
            macro_rules! rename {
                ($field:expr) => {
                    ik_json.as_mut().unwrap()[$field] =
                        ik_json.as_ref().unwrap()[format!("ik_{}", $field)].clone();
                };
            }

            // rename original IK fields, by removing the 'ik_' prefix
            rename!("constraint");
            rename!("mode");
            rename!("target_id");
            rename!("bone_ids");

            // with the renamed fields, reparsing IK json will work
            let ik_str = ik_json.unwrap().to_string();
            let mut des = serde_json::Deserializer::from_str(&ik_str);
            let ik: Result<InverseKinematics, _> = serde_path_to_error::deserialize(&mut des);

            let len = root.as_mut().unwrap().inverse_kinematics.len() as i32;
            for id in &ik.as_ref().unwrap().bone_ids {
                root.as_mut().unwrap().bones[*id as usize].ik_family_id = len;
            }

            root.as_mut().unwrap().inverse_kinematics.push(ik.unwrap());
        }

        // extract physics fields from this bone
        let mut deserializer = serde_json::Deserializer::from_str(&json_string);
        let mut phys_json: Result<Value, _> = serde_path_to_error::deserialize(&mut deserializer);

        macro_rules! rename {
            ($field:expr) => {
                if let Some(field) = phys_json.as_ref().unwrap().get(format!("phys_{}", $field)) {
                    phys_json.as_mut().unwrap()[$field] = field.clone();
                }
            };
        }

        rename!("pos_damping");
        rename!("rot_damping");
        rename!("scale_damping");
        rename!("sway");
        rename!("rot_bounce");

        let phys_str = phys_json.unwrap().to_string();
        let mut des = serde_json::Deserializer::from_str(&phys_str);
        let physics: Result<Physics, _> = serde_path_to_error::deserialize(&mut des);

        root.as_mut().unwrap().physics.push(physics.unwrap());
        let len = root.as_ref().unwrap().physics.len();
        root.as_mut().unwrap().bones[b].physics_id = len as i32 - 1;
    }

    raw["bones"] = serde_json::to_value(&root.as_ref().unwrap().bones).unwrap();
    raw["visuals"] = serde_json::to_value(&root.as_ref().unwrap().visuals).unwrap();
    raw["inverse_kinematics"] =
        serde_json::to_value(&root.as_ref().unwrap().inverse_kinematics).unwrap();
    raw["physics"] = serde_json::to_value(&root.as_ref().unwrap().physics).unwrap();
    raw["version"] = "v0.6".into();
}

pub fn v0d6_7_8_to_v0d9(raw: &mut serde_json::Value) {
    // initiate Root
    let json_string = serde_json::to_string(&raw).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_string);
    let root: Result<Root, _> = serde_path_to_error::deserialize(&mut deserializer);

    // populate keyframe bone_name based on bone_id
    for a in 0..raw.get("animations").unwrap().as_array().unwrap().len() {
        let anim = &mut get_array!(raw, "animations")[a];
        for k in 0..get_array!(anim, "keyframes").len() {
            let kf = &mut get_array!(anim, "keyframes")[k];
            let bones = &root.as_ref().unwrap().bones;
            let name = &bones.iter().find(|b| b.id == kf["bone_id"]);
            if name.is_none() {
                continue;
            }
            kf["bone_name"] = serde_json::Value::String(name.unwrap().name.to_string());
            kf["bone_id"] = json!(-1);
        }
    }
    raw["version"] = "v0.9".into();
}
