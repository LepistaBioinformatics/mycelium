use crate::openapi_processor::{
    build_operation_id,
    load_operations_from_downstream_services::ServiceOpenApiSchema,
};

use actix_web::web;
use myc_http_tools::Profile;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolOperationResponse {
    #[serde(flatten)]
    pub operation: serde_json::Value,

    /// The search match score
    ///
    /// The score of the search match. Higher is better.
    ///
    #[serde(default)]
    pub score: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchOperationResponse {
    /// The operations
    ///
    /// The operations of the search.
    ///
    pub records: Vec<ToolOperationResponse>,

    /// The total
    ///
    /// The total number of operations that match the search.
    ///
    #[serde(default)]
    pub count: usize,

    /// The page size
    ///
    /// The page size of the search.
    ///
    #[serde(default)]
    pub page_size: usize,

    /// The skip
    ///
    /// The skip of the search.
    ///
    #[serde(default)]
    pub skip: usize,
}

#[tracing::instrument(
    name = "list_operations",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, operations_database)
)]
pub(crate) async fn list_operations(
    profile: Profile,
    query: Option<String>,
    method: Option<String>,
    score_cutoff: Option<usize>,
    page_size: Option<usize>,
    skip: Option<usize>,
    operations_database: web::Data<ServiceOpenApiSchema>,
) -> Result<SearchOperationResponse, MappedErrors> {
    let span = tracing::info_span!(
        "search_operation",
        query = ?query,
        method = ?method,
        score_cutoff = ?score_cutoff,
        page_size = ?page_size,
        skip = ?skip
    );

    tracing::debug!("Searching for operations");

    let _guard = span.enter();

    let max_resolution_iterations = 3;
    let score_cutoff = score_cutoff.unwrap_or(5);
    let page_size = page_size.unwrap_or(5);
    let skip = skip.unwrap_or(0);
    let splitted_query = if let Some(query) = query.clone() {
        query
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
    } else {
        vec![]
    };

    let operations_database = operations_database.clone();
    let mut operations = operations_database.operations.clone();
    operations.sort_by(|a, b| {
        a.operation
            .operation_id
            .cmp(&b.operation.operation_id)
            .then(a.method.to_string().cmp(&b.method.to_string()))
            .then(a.path.cmp(&b.path))
            .then(a.service.name.cmp(&b.service.name))
    });

    let mut mut_operations = operations
        .iter()
        .filter(|tool_operation| {
            //
            // Filter by method
            //
            if let Some(method) = &method {
                tool_operation.method.to_string().to_lowercase()
                    == method.to_string().to_lowercase()
            } else {
                true
            }
        })
        //
        // Apply filters
        //
        .filter_map(|tool_operation| {
            let mut realized_matches = vec![];

            //
            // Check if the service name contains the query
            //
            let service_name_contains = splitted_query
                .iter()
                .map(|q| get_match_weight(q, &tool_operation.service.name))
                .collect::<Vec<i32>>();

            realized_matches.extend(service_name_contains);

            //
            // Check if the summary contains the query
            //
            if let Some(summary) = &tool_operation.operation.summary {
                let summary_contains = splitted_query
                    .iter()
                    .map(|q| get_match_weight(q, &summary))
                    .collect::<Vec<i32>>();

                realized_matches.extend(summary_contains);
            }

            //
            // Check if the tags contains the query
            //
            let tags_contains = splitted_query
                .iter()
                .map(|q| {
                    tool_operation
                        .operation
                        .tags
                        .iter()
                        .map(|tag| get_match_weight(q, &tag))
                })
                .flatten()
                .collect::<Vec<i32>>();

            realized_matches.extend(tags_contains);

            //
            // Check if the path contains the query
            //
            let path_contains = splitted_query
                .iter()
                .map(|q| get_match_weight(q, &tool_operation.path))
                .collect::<Vec<i32>>();

            realized_matches.extend(path_contains);

            //
            // Calculate the matching score
            //
            // The score varies from 0 to N. 0 means no match, N means
            // perfect match.
            //
            let expected_matches = realized_matches.len() as i32;

            let observed_matches =
                realized_matches.iter().map(|&b| b).sum::<i32>();

            let score = if expected_matches > 0 {
                (observed_matches * 100) / expected_matches
            } else {
                0
            };

            Some((tool_operation, score))
        })
        //
        // Filter by score
        //
        .filter(|(_, score)| score.to_owned() >= score_cutoff as i32)
        .collect::<Vec<_>>();

    mut_operations.sort_by(|(_, a), (_, b)| b.cmp(&a));

    let records = mut_operations
        .iter()
        .skip(skip)
        .take(page_size)
        .filter_map(|(tool_operation, score)| {
            let operation_id = build_operation_id(
                tool_operation.operation.operation_id.clone(),
                &tool_operation.service.name,
                &tool_operation.path,
            );

            let first_level_resolved_operation = operations_database
                .docs
                .get(&tool_operation.service.name)
                .and_then(|doc| {
                    let inner_operation = tool_operation.operation.clone();

                    let mut serde_tool_operation =
                        match serde_json::to_value(tool_operation) {
                            Ok(doc) => doc,
                            Err(_) => return None,
                        };

                    let resolved_doc = match inner_operation.operation_id {
                        None => Some(serde_json::to_value(doc).unwrap()),
                        Some(operation_id) => {
                            let res = doc.resolve_input_refs_from_operation_id(
                                &operation_id,
                            );

                            match res {
                                Ok(operation) => Some(operation),
                                Err(_) => {
                                    Some(serde_json::to_value(doc).unwrap())
                                }
                            }
                        }
                    };

                    if let Some(resolved_doc) = resolved_doc {
                        //
                        // Merge the resolved operation with the tool operation
                        //
                        // Keys of the resolved_doc should exist at the root of
                        // the tools_operation object
                        //
                        serde_tool_operation
                            .as_object_mut()
                            .unwrap()
                            .extend(resolved_doc.as_object().unwrap().clone());

                        Some(serde_tool_operation)
                    } else {
                        None
                    }
                })
                .ok_or(execution_err("Operation not found"));

            let first_level_resolved_operation =
                if let Ok(value) = first_level_resolved_operation {
                    value
                } else {
                    return None;
                };

            let serde_docs =
                serde_json::to_value(operations_database.docs.clone()).unwrap();

            let mut resolved_operation = first_level_resolved_operation.clone();

            //
            // Resolve the refs recursively
            //
            // Resolution should stop when no more refs are found or the
            // maximum number of iterations is reached.
            //
            for _ in 0..max_resolution_iterations {
                resolved_operation = match edit_refs(
                    &mut resolved_operation.clone(),
                    &edit_fn,
                    0,
                    15,
                    &serde_docs,
                ) {
                    Ok(resolved_operation) => resolved_operation,
                    Err(_) => resolved_operation,
                };
            }

            resolved_operation["operationId"] = operation_id.into();

            Some(ToolOperationResponse {
                operation: resolved_operation,
                score: score.to_owned(),
            })
        })
        .collect::<Vec<_>>();

    Ok(SearchOperationResponse {
        count: records.len(),
        page_size,
        skip,
        records,
    })
}

/// Get the match weight
///
/// If match ie exact, return 2 otherwise return 1, if has no match return 0.
///
fn get_match_weight<T: ToString>(query: &T, subject: &T) -> i32 {
    let query = query.to_string().to_lowercase();
    let subject = subject.to_string().to_lowercase();

    if query == subject {
        return 2;
    }

    if query.contains(&subject) || subject.contains(&query) {
        return 1;
    }

    return 0;
}

/// Edit the refs
///
/// This is the edit function for the refs.
///
fn edit_fn(ref_path: &str, components: &Value) -> Result<String, MappedErrors> {
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

/// Edit the refs
///
/// Resolve the ref path to the actual value.
///
fn edit_refs(
    value: &mut Value,
    edit_fn: &dyn Fn(&str, &Value) -> Result<String, MappedErrors>,
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
                        *s = edit_fn(s, components)?;
                    }

                    continue;
                }

                edit_refs(
                    v,
                    edit_fn,
                    current_depth + 1,
                    max_depth,
                    components,
                )?;
            }
        }
        Value::Array(arr) => {
            for item in arr {
                edit_refs(
                    item,
                    edit_fn,
                    current_depth + 1,
                    max_depth,
                    components,
                )?;
            }
        }
        _ => {}
    }

    Ok(value.clone())
}
