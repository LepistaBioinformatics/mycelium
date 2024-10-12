use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, Hash, PartialEq,
)]
#[serde(rename_all = "camelCase")]
pub enum TenantStatus {
    Verified {
        verified: bool,
        at: DateTime<Local>,
        by: String,
    },

    Trashed {
        trashed: bool,
        at: DateTime<Local>,
        by: String,
    },

    Archived {
        archived: bool,
        at: DateTime<Local>,
        by: String,
    },
}
