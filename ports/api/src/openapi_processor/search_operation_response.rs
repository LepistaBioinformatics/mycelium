use serde::{Deserialize, Serialize};

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
