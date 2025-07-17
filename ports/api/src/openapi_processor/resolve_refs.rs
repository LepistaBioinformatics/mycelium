use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde_json::Value;

/// Resolve the refs
///
/// Resolve the ref path to the actual value.
///
pub(crate) fn resolve_refs(
    value: &mut Value,
    current_depth: usize,
    max_depth: usize,
    components: &Value,
) -> Result<Value, MappedErrors> {
    if current_depth > max_depth {
        return Ok(value.clone());
    }

    match value {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                if k == "$ref" {
                    if let Value::String(s) = v {
                        *s = resolver_fn(s, components)?;
                    }

                    continue;
                }

                resolve_refs(v, current_depth + 1, max_depth, components)?;
            }
        }
        Value::Array(arr) => {
            for item in arr {
                resolve_refs(item, current_depth + 1, max_depth, components)?;
            }
        }
        _ => {}
    }

    Ok(value.clone())
}

/// Resolver function
///
/// This is the resolver function for the refs.
///
fn resolver_fn(
    ref_path: &str,
    components: &Value,
) -> Result<String, MappedErrors> {
    let splitted_ref_path = ref_path.split('/').collect::<Vec<&str>>();
    let default_string = String::new();

    let element_definition = if splitted_ref_path.len() > 2 {
        splitted_ref_path[splitted_ref_path.len() - 2]
    } else {
        return execution_err(format!(
            "Failed to resolve schema ref. Unable to get the component name from reference: {:?}",
            ref_path
        )).as_error();
    };

    let element_name = splitted_ref_path.last().ok_or(execution_err(format!(
        "Failed to resolve schema ref. Unable to get the component name from reference: {:?}",
        default_string
    )))?;

    let ref_value = components
        .get(element_definition)
        .and_then(|schema| schema.get(element_name))
        .ok_or(
            execution_err(format!("Failed to resolve schema ref: {ref_path}"))
                .with_exp_true(),
        )?;

    Ok(ref_value.to_string())
}
