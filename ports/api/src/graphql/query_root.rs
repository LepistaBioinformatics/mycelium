use crate::{
    dtos::ToolOperation,
    graphql::{
        Example, Header, Parameter, RequestBody, Response, Schema,
        ToolsRegistry,
    },
};

use async_graphql::{Context, Object, SimpleObject};
use serde::{Deserialize, Serialize};

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct ToolOperationResponse {
    #[serde(flatten)]
    operation: ToolOperation,

    /// The search match score
    ///
    /// The score of the search match. Higher is better.
    ///
    #[serde(default)]
    score: i32,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct SearchOperationResponse {
    /// The operations
    ///
    /// The operations of the search.
    ///
    #[serde(flatten)]
    operations: Vec<ToolOperationResponse>,

    /// The total
    ///
    /// The total number of operations that match the search.
    ///
    #[serde(default)]
    total: usize,

    /// The page size
    ///
    /// The page size of the search.
    ///
    #[serde(default)]
    page_size: usize,

    /// The skip
    ///
    /// The skip of the search.
    ///
    #[serde(default)]
    skip: usize,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct GetSchemaResponse {
    #[serde(flatten)]
    schema: Option<Schema>,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct GetResponseResponse {
    #[serde(flatten)]
    schema: Option<Response>,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct GetParameterResponse {
    #[serde(flatten)]
    schema: Option<Parameter>,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct GetRequestBodyResponse {
    #[serde(flatten)]
    schema: Option<RequestBody>,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct GetHeaderResponse {
    #[serde(flatten)]
    schema: Option<Header>,
}

#[derive(SimpleObject, Serialize, Deserialize, Clone, Debug)]
struct GetExampleResponse {
    #[serde(flatten)]
    schema: Option<Example>,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get the operation by operation id
    ///
    /// Return a single operation by its operation id.
    ///
    #[tracing::instrument(name = "get_operation", skip_all)]
    async fn get_operation(
        &self,
        ctx: &Context<'_>,
        operation_id: String,
    ) -> Option<ToolOperation> {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let operations = registry.operations.clone();

        let operation = operations
            .iter()
            .find(|op| op.operation_id == operation_id)
            .cloned();

        if let Some(operation) = operation {
            Some(operation)
        } else {
            None
        }
    }

    /// Get schema by name
    ///
    /// If available, return the schema by name. Otherwise, return None.
    ///
    #[tracing::instrument(name = "get_schema", skip_all)]
    async fn get_schema(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> GetSchemaResponse {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let components = registry.components.clone();

        let name =
            if let Some(name) = name.split("/").collect::<Vec<&str>>().last() {
                name.to_owned()
            } else {
                return GetSchemaResponse { schema: None };
            };

        let schema = components
            .iter()
            .map(|c| c.schemas.clone())
            .find_map(|s| s.get(name).map(|s| s.clone()));

        if let Some(schema) = schema {
            if let Some(schema) = schema.schema {
                return GetSchemaResponse {
                    schema: Some(schema),
                };
            }

            //
            // This is a recursive call to get the schema of the reference.
            // Don't unbox it to avoid infinite recursion.
            //
            let introspected_schema = Box::pin(self.get_schema(
                ctx,
                schema.reference.reference.clone().unwrap_or("".to_string()),
            ))
            .await;

            if let Ok(introspected_schema) = introspected_schema {
                return introspected_schema;
            }
        }

        GetSchemaResponse { schema: None }
    }

    /// Get response by name
    ///
    /// If available, return the response by name. Otherwise, return None.
    ///
    #[tracing::instrument(name = "get_response", skip_all)]
    async fn get_response(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> GetResponseResponse {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let components = registry.components.clone();

        let name =
            if let Some(name) = name.split("/").collect::<Vec<&str>>().last() {
                name.to_owned()
            } else {
                return GetResponseResponse { schema: None };
            };

        let schema = components
            .iter()
            .map(|c| c.responses.clone())
            .find_map(|s| s.get(name).map(|s| s.clone()));

        if let Some(schema) = schema {
            if let Some(schema) = schema.schema {
                return GetResponseResponse {
                    schema: Some(schema),
                };
            }

            //
            // This is a recursive call to get the schema of the reference.
            // Don't unbox it to avoid infinite recursion.
            //
            let introspected_schema = Box::pin(self.get_response(
                ctx,
                schema.reference.reference.clone().unwrap_or("".to_string()),
            ))
            .await;

            if let Ok(introspected_schema) = introspected_schema {
                return introspected_schema;
            }
        }

        GetResponseResponse { schema: None }
    }

    /// Get parameter by name
    ///
    /// If available, return the parameter by name. Otherwise, return None.
    ///
    #[tracing::instrument(name = "get_parameter", skip_all)]
    async fn get_parameter(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> GetParameterResponse {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let components = registry.components.clone();

        let name =
            if let Some(name) = name.split("/").collect::<Vec<&str>>().last() {
                name.to_owned()
            } else {
                return GetParameterResponse { schema: None };
            };

        let schema = components
            .iter()
            .map(|c| c.parameters.clone())
            .find_map(|s| s.get(name).map(|s| s.clone()));

        if let Some(schema) = schema {
            if let Some(schema) = schema.schema {
                return GetParameterResponse {
                    schema: Some(schema),
                };
            }

            //
            // This is a recursive call to get the schema of the reference.
            // Don't unbox it to avoid infinite recursion.
            //
            let introspected_schema = Box::pin(self.get_parameter(
                ctx,
                schema.reference.reference.clone().unwrap_or("".to_string()),
            ))
            .await;

            if let Ok(introspected_schema) = introspected_schema {
                return introspected_schema;
            }
        }

        GetParameterResponse { schema: None }
    }

    /// Get request body by name
    ///
    /// If available, return the request body by name. Otherwise, return None.
    ///
    #[tracing::instrument(name = "get_request_body", skip_all)]
    async fn get_request_body(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> GetRequestBodyResponse {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let components = registry.components.clone();

        let name =
            if let Some(name) = name.split("/").collect::<Vec<&str>>().last() {
                name.to_owned()
            } else {
                return GetRequestBodyResponse { schema: None };
            };

        let schema = components
            .iter()
            .map(|c| c.request_bodies.clone())
            .find_map(|s| s.get(name).map(|s| s.clone()));

        if let Some(schema) = schema {
            if let Some(schema) = schema.schema {
                return GetRequestBodyResponse {
                    schema: Some(schema),
                };
            }

            //
            // This is a recursive call to get the schema of the reference.
            // Don't unbox it to avoid infinite recursion.
            //
            let introspected_schema = Box::pin(self.get_request_body(
                ctx,
                schema.reference.reference.clone().unwrap_or("".to_string()),
            ))
            .await;

            if let Ok(introspected_schema) = introspected_schema {
                return introspected_schema;
            }
        }

        GetRequestBodyResponse { schema: None }
    }

    /// Get header by name
    ///
    /// If available, return the header by name. Otherwise, return None.
    ///
    #[tracing::instrument(name = "get_header", skip_all)]
    async fn get_header(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> GetHeaderResponse {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let components = registry.components.clone();

        let name =
            if let Some(name) = name.split("/").collect::<Vec<&str>>().last() {
                name.to_owned()
            } else {
                return GetHeaderResponse { schema: None };
            };

        let schema = components
            .iter()
            .map(|c| c.headers.clone())
            .find_map(|s| s.get(name).map(|s| s.clone()));

        if let Some(schema) = schema {
            if let Some(schema) = schema.schema {
                return GetHeaderResponse {
                    schema: Some(schema),
                };
            }

            //
            // This is a recursive call to get the schema of the reference.
            // Don't unbox it to avoid infinite recursion.
            //
            let introspected_schema = Box::pin(self.get_header(
                ctx,
                schema.reference.reference.clone().unwrap_or("".to_string()),
            ))
            .await;

            if let Ok(introspected_schema) = introspected_schema {
                return introspected_schema;
            }
        }

        GetHeaderResponse { schema: None }
    }

    /// Get example by name
    ///
    /// If available, return the example by name. Otherwise, return None.
    ///
    #[tracing::instrument(name = "get_example", skip_all)]
    async fn get_example(
        &self,
        ctx: &Context<'_>,
        name: String,
    ) -> GetExampleResponse {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let components = registry.components.clone();

        let name =
            if let Some(name) = name.split("/").collect::<Vec<&str>>().last() {
                name.to_owned()
            } else {
                return GetExampleResponse { schema: None };
            };

        let schema = components
            .iter()
            .map(|c| c.examples.clone())
            .find_map(|s| s.get(name).map(|s| s.clone()));

        if let Some(schema) = schema {
            if let Some(schema) = schema.schema {
                return GetExampleResponse {
                    schema: Some(schema),
                };
            }

            //
            // This is a recursive call to get the schema of the reference.
            // Don't unbox it to avoid infinite recursion.
            //
            let introspected_schema = Box::pin(self.get_example(
                ctx,
                schema.reference.reference.clone().unwrap_or("".to_string()),
            ))
            .await;

            if let Ok(introspected_schema) = introspected_schema {
                return introspected_schema;
            }
        }

        GetExampleResponse { schema: None }
    }

    /// Search for operations
    ///
    /// Search for operations by query, method, service name, and score cutoff.
    ///
    #[tracing::instrument(name = "search_operation", skip_all)]
    async fn search_operation(
        &self,
        ctx: &Context<'_>,
        query: String,
        method: Option<String>,
        score_cutoff: Option<i32>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> SearchOperationResponse {
        let registry = ctx.data_unchecked::<ToolsRegistry>();

        let score_cutoff = score_cutoff.unwrap_or(0);
        let page_size = page_size.unwrap_or(5);
        let skip = skip.unwrap_or(0);
        let splitted_query = query.split_whitespace().collect::<Vec<&str>>();

        let mut operations = registry.operations.clone();
        operations.sort_by(|a, b| {
            a.operation_id
                .cmp(&b.operation_id)
                .then(a.path.cmp(&b.path))
                .then(a.method.cmp(&b.method))
        });

        let mut filtered_operations = operations
            .iter()
            .filter(|op| {
                //
                // Check if the method contains the query
                //
                if let Some(method) = method.clone() {
                    op.method.to_lowercase().contains(&method.to_lowercase())
                } else {
                    true
                }
            })
            .map(|op| {
                let mut realized_matches = vec![];

                //
                // Check if the service name contains the query
                //
                let service_name_contains = splitted_query
                    .iter()
                    .map(|q| {
                        get_match_weight(
                            q,
                            &op.service.name.to_lowercase().as_str(),
                        )
                    })
                    .collect::<Vec<i32>>();

                realized_matches.extend(service_name_contains);

                //
                // Check if the summary contains the query
                //
                let summary_contains = splitted_query
                    .iter()
                    .map(|q| {
                        get_match_weight(
                            q,
                            &op.summary
                                .as_deref()
                                .unwrap_or("")
                                .to_lowercase()
                                .as_str(),
                        )
                    })
                    .collect::<Vec<i32>>();

                realized_matches.extend(summary_contains);

                //
                // Check if the tags contains the query
                //
                let tags_contains = splitted_query
                    .iter()
                    .map(|q| {
                        op.operation.tags.iter().map(|tag| {
                            get_match_weight(q, &tag.to_lowercase().as_str())
                        })
                    })
                    .flatten()
                    .collect::<Vec<i32>>();

                realized_matches.extend(tags_contains);

                //
                // Check if the path contains the query
                //
                let path_contains = splitted_query
                    .iter()
                    .map(|q| {
                        get_match_weight(q, &op.path.to_lowercase().as_str())
                    })
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

                let score = (observed_matches * 100) / expected_matches;

                ToolOperationResponse {
                    operation: op.clone(),
                    score,
                }
            })
            .filter(|op| op.score > score_cutoff)
            .collect::<Vec<_>>();

        filtered_operations.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap()
                .then(a.operation.operation_id.cmp(&b.operation.operation_id))
        });

        SearchOperationResponse {
            total: filtered_operations.len(),
            page_size: page_size as usize,
            skip: skip as usize,
            operations: filtered_operations
                .clone()
                .into_iter()
                .skip(skip as usize)
                .take(page_size as usize)
                .collect(),
        }
    }
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
