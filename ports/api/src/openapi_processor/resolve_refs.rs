use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde_json::{Map, Value};

const COMPONENTS_REFERENCE_PREFIX: &str = "#/components/";

/// Resolve the refs
///
/// Replace, in place, every `$ref` pointing to `#/components/` with the
/// referenced component of the document. A replaced reference is not
/// traversed again in the same pass, so nested references need another pass.
///
pub(crate) fn resolve_refs(
    value: &mut Value,
    current_depth: usize,
    max_depth: usize,
    document: &Value,
) -> Result<(), MappedErrors> {
    if current_depth > max_depth {
        return Ok(());
    }

    match value {
        Value::Object(fields) => {
            resolve_object_refs(fields, current_depth, max_depth, document)
        }
        Value::Array(items) => items.iter_mut().try_for_each(|item| {
            resolve_refs(item, current_depth + 1, max_depth, document)
        }),
        _ => Ok(()),
    }
}

fn resolve_object_refs(
    fields: &mut Map<String, Value>,
    current_depth: usize,
    max_depth: usize,
    document: &Value,
) -> Result<(), MappedErrors> {
    for (key, field) in fields.iter_mut() {
        if let Some(reference) = component_reference(key, field) {
            *field = resolve_component_reference(reference, document)?;
            continue;
        }

        resolve_refs(field, current_depth + 1, max_depth, document)?;
    }

    Ok(())
}

fn component_reference<'a>(key: &str, field: &'a Value) -> Option<&'a str> {
    if key != "$ref" {
        return None;
    }

    field
        .as_str()
        .filter(|reference| reference.starts_with(COMPONENTS_REFERENCE_PREFIX))
}

/// Resolve a component reference
///
/// Look up `#/components/<definition>/<name>` in the document.
///
fn resolve_component_reference(
    reference: &str,
    document: &Value,
) -> Result<Value, MappedErrors> {
    let segments = reference.split('/').collect::<Vec<&str>>();

    let [_, .., element_definition, element_name] = segments.as_slice() else {
        return execution_err(format!(
            "Failed to resolve schema ref. Unable to get the component name \
             from reference: {reference:?}"
        ))
        .as_error();
    };

    document
        .get("components")
        .and_then(|components| components.get(element_definition))
        .and_then(|definitions| definitions.get(element_name))
        .cloned()
        .ok_or(
            execution_err(format!("Failed to resolve schema ref: {reference}"))
                .with_exp_true(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn build_document() -> Value {
        json!({
            "components": {
                "schemas": {
                    "Record": {
                        "type": "object",
                        "properties": {
                            "child": { "$ref": "#/components/schemas/Child" }
                        }
                    },
                    "Child": { "type": "integer" }
                }
            }
        })
    }

    #[test]
    fn resolve_refs_replaces_one_reference_level_per_pass(
    ) -> Result<(), MappedErrors> {
        let document = build_document();
        let mut value = json!([{ "$ref": "#/components/schemas/Record" }]);

        resolve_refs(&mut value, 0, 15, &document)?;
        assert_eq!(
            value[0]["$ref"]["properties"]["child"]["$ref"],
            "#/components/schemas/Child"
        );

        resolve_refs(&mut value, 0, 15, &document)?;
        assert_eq!(
            value[0]["$ref"]["properties"]["child"]["$ref"]["type"],
            "integer"
        );

        Ok(())
    }

    #[test]
    fn resolve_refs_fails_on_unknown_component() {
        let document = build_document();
        let mut value = json!({ "$ref": "#/components/schemas/Missing" });

        assert!(resolve_refs(&mut value, 0, 15, &document).is_err());
    }

    #[test]
    fn resolve_refs_stops_beyond_max_depth() -> Result<(), MappedErrors> {
        let document = build_document();
        let mut value =
            json!({ "nested": { "$ref": "#/components/schemas/Child" } });

        resolve_refs(&mut value, 0, 0, &document)?;

        assert_eq!(value["nested"]["$ref"], "#/components/schemas/Child");

        Ok(())
    }
}
