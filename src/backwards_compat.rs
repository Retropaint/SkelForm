use crate::Root;

pub fn proceed(mut raw: serde_json::Value) -> (Root, String) {
    let version_raw = raw.get("version").unwrap().clone();
    let version = version_raw.as_str().unwrap();
    let mut err = "".to_string();

    macro_rules! from_ver {
        ($ver:expr, $func:ident) => {
            if version.contains($ver) {
                $func(&mut raw);
                let json_string = serde_json::to_string(&raw).unwrap();
                let mut deserializer = serde_json::Deserializer::from_str(&json_string);
                let result: Result<Root, _> = serde_path_to_error::deserialize(&mut deserializer);
                if let Err(invalid) = result {
                    err = "from v".to_owned() + $ver + ": \n\n" + &invalid.to_string();
                }
            }
        };
    }

    from_ver!("0.2", v0d2_to_v0d3);
    from_ver!("0.3", v0d3_to_v0d4);

    if err == "" {
        return (serde_json::from_value(raw).unwrap(), "".to_string());
    } else {
        return (Root::default(), err);
    }
}

pub fn v0d2_to_v0d3(raw: &mut serde_json::Value) {
    raw["version"] = "v0.4.0".into();
}

pub fn v0d3_to_v0d4(raw: &mut serde_json::Value) {
    for bone in raw.get_mut("bones").unwrap().as_array_mut().unwrap() {
        if let Some(str) = bone.get("ik_constraint_str") {
            bone["ik_constraint"] = str.clone();
        }
        if let Some(str) = bone.get("ik_mode_str") {
            bone["ik_mode"] = str.clone();
        }
    }
    for anim in raw.get_mut("animations").unwrap().as_array_mut().unwrap() {
        for keyframe in anim.get_mut("keyframes").unwrap().as_array_mut().unwrap() {
            keyframe["element"] = keyframe["element_str"].clone();
        }
    }
    raw["version"] = "v0.4.0".into();
}
