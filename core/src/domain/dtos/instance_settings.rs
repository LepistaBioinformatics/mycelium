// ? ---------------------------------------------------------------------------
// ? InstanceSetting
//
// One row of the generalized `instance_settings` key-value store. Each row
// is a single named configuration entry, keyed by an application-defined
// string constant; `value`'s shape is defined and validated entirely by the
// use-case layer that owns that key -- never by this DTO or the persistence
// layer. This is what lets the same table back arbitrary future
// instance-wide settings, not just the staff bootstrap flag below.
// ? ---------------------------------------------------------------------------

use crate::domain::dtos::written_by::WrittenBy;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

/// Key of the staff-bootstrap claim entry. Row *presence* under this key is
/// itself the "already claimed" signal -- absence means bootstrap is still
/// pending. No `status` field is stored anywhere; there is nothing to drift
/// out of sync with row existence. Who claimed it and when are already
/// carried by `created_by`/`created` below -- no separate payload needed.
pub const STAFF_BOOTSTRAP_KEY: &str = "staff_bootstrap";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSetting {
    pub key: String,
    pub value: serde_json::Value,
    pub created_by: Option<WrittenBy>,
    pub updated_by: Option<WrittenBy>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::Uuid;

    #[test]
    fn instance_setting_round_trips_through_json() {
        let setting = InstanceSetting {
            key: STAFF_BOOTSTRAP_KEY.to_string(),
            value: serde_json::json!({}),
            created_by: Some(WrittenBy::new_from_user_with_email(
                Uuid::new_v4(),
                "staff@example.com",
            )),
            updated_by: None,
            created: Local::now(),
            updated: None,
        };

        let json = serde_json::to_string(&setting).unwrap();
        let parsed: InstanceSetting = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.key, STAFF_BOOTSTRAP_KEY);
        assert!(parsed.created_by.is_some());
    }
}
