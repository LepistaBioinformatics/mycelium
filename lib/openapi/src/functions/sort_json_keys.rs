use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Sort the keys of a JSON object
///
/// This is used to make the JSON object deterministic.
///
pub fn sort_json_keys(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted_map = BTreeMap::new();
            for (key, val) in map {
                sorted_map.insert(key, sort_json_keys(val));
            }
            Value::Object(Map::from_iter(sorted_map))
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(sort_json_keys).collect())
        }
        _ => value,
    }
}
