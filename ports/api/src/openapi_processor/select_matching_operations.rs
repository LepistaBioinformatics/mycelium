use crate::{
    dtos::ToolOperation,
    openapi_processor::{
        operation_filters::OperationFilters,
        score_operation_match::score_operation_match,
        service_openapi_schema::ServiceOpenApiSchema,
    },
};

const DEFAULT_SCORE_CUTOFF: usize = 5;

/// Select the operations matching the filters
///
/// Without query terms every operation passing the method and origin filters
/// is selected, in catalog order, with score 0. With query terms only
/// operations scoring at least the score cutoff are selected, best score
/// first (ties keep catalog order).
///
pub(crate) fn select_matching_operations<'catalog>(
    catalog: &'catalog ServiceOpenApiSchema,
    filters: &OperationFilters,
) -> Vec<(&'catalog ToolOperation, i32)> {
    let candidates = catalog
        .operations
        .iter()
        .filter(|tool_operation| filters.admits(tool_operation));

    let query_terms = filters.query_terms();

    if query_terms.is_empty() {
        return candidates
            .map(|tool_operation| (tool_operation, 0))
            .collect();
    }

    let score_cutoff = filters.score_cutoff.unwrap_or(DEFAULT_SCORE_CUTOFF);
    let minimum_score = i32::try_from(score_cutoff).unwrap_or(i32::MAX);

    let mut scored = candidates
        .map(|tool_operation| {
            (
                tool_operation,
                score_operation_match(tool_operation, &query_terms),
            )
        })
        .filter(|(_, score)| *score >= minimum_score)
        .collect::<Vec<_>>();

    scored.sort_by(|(_, first), (_, second)| second.cmp(first));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi_processor::synthetic_catalog_fixture::build_synthetic_catalog;
    use mycelium_base::utils::errors::MappedErrors;

    fn build_catalog() -> Result<ServiceOpenApiSchema, MappedErrors> {
        let catalog = build_synthetic_catalog(2, 2);
        ServiceOpenApiSchema::new(catalog.operations, catalog.docs)
    }

    fn filters(
        method: Option<&str>,
        exclude_gateway: bool,
    ) -> OperationFilters {
        OperationFilters {
            method: method.map(str::to_string),
            exclude_gateway,
            ..Default::default()
        }
    }

    #[test]
    fn method_filter_is_case_insensitive() -> Result<(), MappedErrors> {
        let catalog = build_catalog()?;

        assert_eq!(
            select_matching_operations(&catalog, &filters(Some("post"), false))
                .len(),
            4
        );
        assert!(select_matching_operations(
            &catalog,
            &filters(Some("GET"), false)
        )
        .is_empty());

        Ok(())
    }

    #[test]
    fn excludes_the_gateway_operations_on_request() -> Result<(), MappedErrors>
    {
        let mut catalog = build_synthetic_catalog(2, 2);
        let gateway_operation = catalog
            .operations
            .first_mut()
            .expect("the synthetic catalog has operations");
        gateway_operation.service.name =
            crate::openapi_processor::GATEWAY_CATALOG_SERVICE_NAME.to_string();
        let catalog =
            ServiceOpenApiSchema::new(catalog.operations, catalog.docs)?;

        assert_eq!(
            select_matching_operations(&catalog, &filters(None, false)).len(),
            4
        );
        assert_eq!(
            select_matching_operations(&catalog, &filters(None, true)).len(),
            3
        );

        Ok(())
    }
}
