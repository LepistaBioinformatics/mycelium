use schemars::{generate::SchemaSettings, JsonSchema};
use std::borrow::Cow;

/// Recursively removes keys with `null` value from the schema. Tools like the
/// OpenRPC generator titleizer call `Object.entries()` on schema nodes and fail
/// when the value is `null`.
fn remove_null_values(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.retain(|_, v| {
            remove_null_values(v);
            !v.is_null()
        });
    } else if let Some(arr) = value.as_array_mut() {
        for v in arr.iter_mut() {
            remove_null_values(v);
        }
    }
}

/// Ensures every node that looks like a JSON Schema has the keys the
/// open-rpc-generator titleizer (@json-schema-tools/titleizer) expects:
/// - `properties` when it has `type` (avoids Object.entries(schema.properties)
///   on undefined);
/// - `items` when it has `type: "array"` (avoids Object.entries(schema.items)
///   on undefined).
fn ensure_schema_safe_for_titleizer(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        if obj.contains_key("type") {
            if !obj.contains_key("properties") {
                obj.insert("properties".into(), serde_json::Map::new().into());
            }
            if obj.get("type").and_then(|t| t.as_str()) == Some("array")
                && !obj.contains_key("items")
            {
                let items =
                    serde_json::json!({ "type": "object", "properties": {} });
                obj.insert("items".into(), items);
            }
        }
        for v in obj.values_mut() {
            ensure_schema_safe_for_titleizer(v);
        }
    } else if let Some(arr) = value.as_array_mut() {
        for v in arr.iter_mut() {
            ensure_schema_safe_for_titleizer(v);
        }
    }
}

pub(crate) fn ensure_schema_safe_for_openrpc_generator(
    spec: &mut serde_json::Value,
) {
    ensure_schema_safe_for_titleizer(spec);
}

/// Generates the param schema in OpenAPI 3.0 format: optional fields use `type:
/// "string"` + `nullable: true` instead of `type: ["string", "null"]`. Uses
/// `/$defs` for subschemas; the leading slash ensures `$ref` in RFC 6901 format
/// (e.g. `#/$defs/RoleParam` and not `#$defs/RoleParam`).
pub fn param_schema_value<T: JsonSchema>() -> serde_json::Value {
    let settings = SchemaSettings::openapi3().with(|s| {
        s.definitions_path = Cow::Borrowed("/$defs");
    });
    let generator = settings.into_generator();
    let root = generator.into_root_schema_for::<T>();
    let mut value = serde_json::to_value(&root)
        .expect("param schema must serialize to JSON");
    remove_null_values(&mut value);
    value
}
