use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A default pagination response
///
/// A paginated record include the total number of records found into a query
/// plus page size which records will be retrieved, the number of records to be
/// ignored (such value should be discovered after the first query), and the
/// records itself.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedRecord<T> {
    pub count: i64,
    pub skip: Option<i64>,
    pub size: Option<i64>,
    pub records: Vec<T>,
}
