use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum RelatedAccounts {
    AllowedAccounts(Vec<Uuid>),
    HasTenantWidePrivileges(Uuid),
    HasStaffPrivileges,
    HasManagerPrivileges,
}
