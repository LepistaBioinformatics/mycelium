//! JSON-RPC param DTOs for gateway manager scope (gatewayManager.routes.*, gatewayManager.services.*, gatewayManager.tools.*).

use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Routes (list routes by service)
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListRoutesParams {
    #[schemars(description = "Filter by service ID")]
    pub id: Option<Uuid>,
    #[schemars(description = "Filter by service name")]
    pub name: Option<String>,
    #[schemars(description = "Page size for pagination")]
    pub page_size: Option<i32>,
    #[schemars(description = "Number of records to skip")]
    pub skip: Option<i32>,
}

// ---------------------------------------------------------------------------
// Services (list services)
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListServicesParams {
    #[schemars(description = "Filter by service ID")]
    pub id: Option<Uuid>,
    #[schemars(description = "Filter by service name")]
    pub name: Option<String>,
    #[schemars(description = "Page size for pagination")]
    pub page_size: Option<i32>,
    #[schemars(description = "Number of records to skip")]
    pub skip: Option<i32>,
}

// ---------------------------------------------------------------------------
// Tools (list operations)
// ---------------------------------------------------------------------------

#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListOperationsParams {
    #[schemars(description = "Search query")]
    pub query: Option<String>,
    #[schemars(description = "Filter by HTTP method")]
    pub method: Option<String>,
    #[schemars(description = "Minimum score cutoff for search matches")]
    pub score_cutoff: Option<usize>,
    #[schemars(description = "Page size for pagination")]
    pub page_size: Option<usize>,
    #[schemars(description = "Number of records to skip")]
    pub skip: Option<usize>,
}
