use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FetchResourceAuditTrailParams {
    #[schemars(
        description = "Resource type: account, accountMeta, user, tenant, tenantMeta, guestRole, webhook"
    )]
    pub resource_type: String,
    pub resource_id: Uuid,
}
