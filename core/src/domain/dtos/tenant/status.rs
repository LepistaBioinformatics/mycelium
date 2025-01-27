use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, Hash, PartialEq,
)]
#[serde(rename_all = "camelCase")]
pub enum TenantStatus {
    Verified { at: DateTime<Local>, by: String },
    Trashed { at: DateTime<Local>, by: String },
    Archived { at: DateTime<Local>, by: String },
}

impl TenantStatus {
    pub fn is_archived(&self) -> bool {
        matches!(self, TenantStatus::Archived { .. })
    }

    pub fn is_trashed(&self) -> bool {
        matches!(self, TenantStatus::Trashed { .. })
    }

    pub fn is_verified(&self) -> bool {
        matches!(self, TenantStatus::Verified { .. })
    }
}
