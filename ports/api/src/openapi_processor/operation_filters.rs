use crate::{
    dtos::ToolOperation,
    openapi_processor::initialize_tools_registry::GATEWAY_CATALOG_SERVICE_NAME,
};

/// The filters a Discovery search applies before scoring and pagination
#[derive(Debug, Default, Clone)]
pub(crate) struct OperationFilters {
    /// HTTP method, compared case-insensitively
    pub method: Option<String>,

    /// Free-text query; each whitespace-separated term is scored
    pub query: Option<String>,

    /// Minimum score of a query match (ignored without a query)
    pub score_cutoff: Option<usize>,

    /// Leave out the gateway's own operations
    pub exclude_gateway: bool,
}

impl OperationFilters {
    /// Whether the operation passes the method and origin filters
    pub(crate) fn admits(&self, tool_operation: &ToolOperation) -> bool {
        self.matches_method(tool_operation)
            && self.matches_origin(tool_operation)
    }

    /// The query split into whitespace-separated terms
    pub(crate) fn query_terms(&self) -> Vec<String> {
        self.query
            .as_deref()
            .map(|query| query.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default()
    }

    fn matches_method(&self, tool_operation: &ToolOperation) -> bool {
        let Some(method) = self.method.as_deref() else {
            return true;
        };

        tool_operation
            .method
            .to_string()
            .eq_ignore_ascii_case(method)
    }

    fn matches_origin(&self, tool_operation: &ToolOperation) -> bool {
        !self.exclude_gateway
            || tool_operation.service.name != GATEWAY_CATALOG_SERVICE_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_terms_ignore_blank_queries() {
        let blank = OperationFilters {
            query: Some("   ".to_string()),
            ..Default::default()
        };
        let spaced = OperationFilters {
            query: Some(" a  b ".to_string()),
            ..Default::default()
        };

        assert!(OperationFilters::default().query_terms().is_empty());
        assert!(blank.query_terms().is_empty());
        assert_eq!(spaced.query_terms(), vec!["a", "b"]);
    }
}
