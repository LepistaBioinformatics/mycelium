//! Helpers to build JSON Schema values from param types (schemars).

use schemars::schema_for;

pub fn param_schema_value<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(&schema_for!(T)).expect("param schema must serialize to JSON")
}
