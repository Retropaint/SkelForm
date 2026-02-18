use strum::FromRepr;

use crate::Root;

pub fn proceed(mut raw: serde_json::Value) -> Root {
    let version = raw.get("version").unwrap().as_str().unwrap();

    if version.contains("v0.3") {
        v0d3_to_v0d4(&mut raw);
    }

    serde_json::from_value(raw).unwrap()
}

pub fn v0d3_to_v0d4(raw: &mut serde_json::Value) {
    for bone in raw.get_mut("bones").iter_mut() {
        bone["hidden"] = bone["is_hidden"].clone();
    }
}
