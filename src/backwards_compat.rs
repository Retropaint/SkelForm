use strum::FromRepr;

use crate::Root;

pub fn proceed(raw: serde_json::Value) -> Root {
    let version = raw.get("version").unwrap().as_str().unwrap();
    serde_json::from_value(raw).unwrap()
}
