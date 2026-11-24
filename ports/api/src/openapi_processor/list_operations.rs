use crate::{
    dtos::ToolOperation,
    openapi_processor::{
        operation_filters::OperationFilters,
        resolve_operation_document::resolve_operation_document,
        search_operation_response::{
            SearchOperationResponse, ToolOperationResponse,
        },
        select_matching_operations::select_matching_operations,
        service_openapi_schema::ServiceOpenApiSchema,
    },
};

use actix_web::web;
use myc_http_tools::Profile;
use mycelium_base::utils::errors::MappedErrors;

const DEFAULT_PAGE_SIZE: usize = 5;

/// List the discoverable operations
///
/// `count` is the number of operations matching the method and query filters
/// before pagination. Without a query every operation is listed in catalog
/// order and `score_cutoff` is ignored.
///
#[tracing::instrument(
    name = "list_operations",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
        filters = ?filters,
        page_size = ?page_size,
        skip = ?skip,
    ),
    skip_all
)]
pub(crate) async fn list_operations(
    profile: Profile,
    filters: OperationFilters,
    page_size: Option<usize>,
    skip: Option<usize>,
    operations_database: web::Data<ServiceOpenApiSchema>,
) -> Result<SearchOperationResponse, MappedErrors> {
    tracing::debug!("Searching for operations");

    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    let skip = skip.unwrap_or(0);

    let matches = select_matching_operations(&operations_database, &filters);

    Ok(SearchOperationResponse {
        count: matches.len(),
        records: build_page_records(
            &matches[skip.min(matches.len())..],
            page_size,
            &operations_database,
        )?,
        page_size,
        skip,
    })
}

fn build_page_records(
    matches: &[(&ToolOperation, i32)],
    page_size: usize,
    catalog: &ServiceOpenApiSchema,
) -> Result<Vec<ToolOperationResponse>, MappedErrors> {
    matches
        .iter()
        .take(page_size)
        .map(|(tool_operation, score)| {
            Ok(ToolOperationResponse {
                operation: resolve_operation_document(tool_operation, catalog)?,
                score: *score,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi_processor::synthetic_catalog_fixture::build_synthetic_catalog;
    use std::time::Instant;
    use uuid::Uuid;

    fn build_profile() -> Profile {
        Profile::new(
            vec![],
            Uuid::nil(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    fn build_database(
        service_count: usize,
        operations_per_service: usize,
    ) -> Result<web::Data<ServiceOpenApiSchema>, MappedErrors> {
        let catalog =
            build_synthetic_catalog(service_count, operations_per_service);

        Ok(web::Data::new(ServiceOpenApiSchema::new(
            catalog.operations,
            catalog.docs,
        )?))
    }

    async fn list(
        database: &web::Data<ServiceOpenApiSchema>,
        query: Option<&str>,
        score_cutoff: Option<usize>,
        page_size: usize,
    ) -> Result<SearchOperationResponse, MappedErrors> {
        let filters = OperationFilters {
            query: query.map(str::to_string),
            score_cutoff,
            ..Default::default()
        };

        list_operations(
            build_profile(),
            filters,
            Some(page_size),
            None,
            database.clone(),
        )
        .await
    }

    #[tokio::test]
    async fn count_is_the_total_of_matches_not_the_page_length(
    ) -> Result<(), MappedErrors> {
        let database = build_database(3, 4)?;

        let response = list(&database, None, None, 5).await?;
        assert_eq!(response.records.len(), 5);
        assert_eq!(response.count, 12);

        let response = list(&database, Some("records"), None, 5).await?;
        assert_eq!(response.records.len(), 5);
        assert_eq!(response.count, 12);

        Ok(())
    }

    #[tokio::test]
    async fn empty_query_lists_every_operation_in_catalog_order(
    ) -> Result<(), MappedErrors> {
        let database = build_database(3, 4)?;

        let response = list(&database, None, None, 100).await?;

        let listed_paths = response
            .records
            .iter()
            .map(|record| record.operation["path"].clone())
            .collect::<Vec<_>>();
        let catalog_paths = database
            .operations
            .iter()
            .map(|operation| serde_json::Value::from(operation.path.clone()))
            .collect::<Vec<_>>();

        assert_eq!(listed_paths, catalog_paths);
        assert!(response.records.iter().all(|record| record.score == 0));

        Ok(())
    }

    #[tokio::test]
    async fn score_cutoff_still_filters_a_query() -> Result<(), MappedErrors> {
        let database = build_database(3, 4)?;

        let response = list(&database, Some("service-02"), None, 100).await?;
        assert_eq!(response.count, 4);
        assert!(response
            .records
            .iter()
            .all(|record| record.operation["service"]["name"] == "service-02"));

        let response =
            list(&database, Some("service-02"), Some(76), 100).await?;
        assert_eq!(response.count, 0);
        assert!(response.records.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn record_never_carries_the_service_document(
    ) -> Result<(), MappedErrors> {
        let mut catalog = build_synthetic_catalog(2, 3);
        catalog.operations[0].operation.operation_id = None;
        catalog.operations[1].operation.operation_id =
            Some("missing_operation".to_string());
        let database = web::Data::new(ServiceOpenApiSchema::new(
            catalog.operations,
            catalog.docs,
        )?);

        let response = list(&database, None, None, 100).await?;

        assert_eq!(response.count, 6);
        for record in response.records {
            assert!(record.operation.get("paths").is_none());
            assert!(record.operation.get("components").is_none());
            assert!(record.operation["operationId"].is_string());
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore = "timing evidence; run with --ignored --nocapture"]
    async fn time_list_operations_on_synthetic_catalog(
    ) -> Result<(), MappedErrors> {
        let database = build_database(30, 40)?;
        let iterations = 10;

        for query in [None, Some("records")] {
            let started = Instant::now();
            let mut response = list(&database, query, None, 20).await?;
            for _ in 1..iterations {
                response = list(&database, query, None, 20).await?;
            }

            println!(
                "query={query:?} average={:?} records={} count={}",
                started.elapsed() / iterations,
                response.records.len(),
                response.count
            );
        }

        Ok(())
    }
}
