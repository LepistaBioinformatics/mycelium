use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, Hash, PartialEq,
)]
#[serde(rename_all = "camelCase")]
pub enum TenantStatus {
    /// If the tenant information is verified
    ///
    /// The verified status is the status of the tenant that indicates if the
    /// tenant is verified.
    Verified(bool),

    /// The date when the tenant was verified
    ///
    /// The verified at status is the status of the tenant that indicates the
    /// date when the tenant was verified.
    VerifiedAt(DateTime<Local>),

    /// The user who verified the tenant
    ///
    /// The verified by status is the status of the tenant that indicates the
    /// user who verified the tenant.
    VerifiedBy(String),

    /// If the tenant is trashed
    ///
    /// The trashed status is the status of the tenant that indicates if the
    /// tenant is trashed.
    Trashed(bool),

    /// The date when the tenant was trashed
    ///
    /// The trashed at status is the status of the tenant that indicates the
    /// date when the tenant was trashed.
    TrashedAt(DateTime<Local>),

    /// The user who trashed the tenant
    ///
    /// The trashed by status is the status of the tenant that indicates the
    /// user who trashed the tenant.
    TrashedBy(String),

    /// If the tenant is archived
    ///
    /// The archived status is the status of the tenant that indicates if the
    /// tenant is archived.
    Archived(bool),

    /// The date when the tenant was archived
    ///
    /// The archived at status is the status of the tenant that indicates the
    /// date when the tenant was archived.
    ArchivedAt(DateTime<Local>),

    /// The user who archived the tenant
    ///
    /// The archived by status is the status of the tenant that indicates the
    /// user who archived the tenant.
    ArchivedBy(String),
}
