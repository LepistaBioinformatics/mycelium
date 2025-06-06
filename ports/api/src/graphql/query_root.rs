use std::cmp::Ordering;

use crate::{dtos::ToolOperation, graphql::ToolsRegistry};

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

pub struct QueryRoot;

#[Object]
impl QueryRoot {
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
                //
                // Check if the summary contains the query
                //
                let summary_contains = splitted_query.iter().any(|q| {
                    op.summary
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(q)
                });

                //
                // Check if the description contains the query
                //
                let description_contains = splitted_query.iter().any(|q| {
                    op.description
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(q)
                });

                //
                // Check if the tags contains the query
                //
                let tags_contains = splitted_query.iter().any(|q| {
                    op.operation
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(q))
                });

                //
                // Check if the path contains the query
                //
                let path_contains = splitted_query
                    .iter()
                    .any(|q| op.path.to_lowercase().contains(q));

                //
                // Check if the capabilities contains the query
                //
                let capabilities_contains =
                    op.service.capabilities.iter().any(|cap| {
                        cap.iter().map(|c| c.to_lowercase()).any(|c| {
                            splitted_query.iter().any(|q| c.contains(q))
                        })
                    });

                //
                // Calculate the matching score
                //
                // The score varies from 0 to 100. 0 means no match, 100 means
                // perfect match.
                //
                let expected_matches = [
                    summary_contains as i32,
                    description_contains as i32,
                    tags_contains as i32,
                    path_contains as i32,
                    capabilities_contains as i32,
                ];

                let observed_matches = expected_matches.iter().sum::<i32>();

                let score =
                    (observed_matches * 100) / expected_matches.len() as i32;

                ToolOperationResponse {
                    operation: op.clone(),
                    score,
                }
            })
            .filter(|op| op.score > score_cutoff)
            .skip(skip as usize)
            .collect::<Vec<_>>();

        filtered_operations.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then(a.operation.operation_id.cmp(&b.operation.operation_id))
        });

        SearchOperationResponse {
            total: filtered_operations.len(),
            page_size: page_size as usize,
            skip: skip as usize,
            operations: filtered_operations
                .clone()
                .into_iter()
                .take(page_size as usize)
                .collect(),
        }
    }
}
