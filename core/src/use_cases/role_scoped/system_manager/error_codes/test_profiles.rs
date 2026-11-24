use crate::domain::dtos::profile::{Owner, Profile};

use std::str::FromStr;
use uuid::Uuid;

/// A profile holding the system-manager role over system accounts
pub(super) fn system_manager_profile() -> Profile {
    let id = Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
        .expect("valid test uuid");

    Profile::new(
        vec![Owner {
            id,
            email: "user-01@domain.com".to_string(),
            first_name: Some("first_name".to_string()),
            last_name: Some("last_name".to_string()),
            username: Some("user-01".to_string()),
            is_principal: true,
        }],
        id,
        false,
        true,
        false,
        true,
        true,
        true,
        false,
        false,
        None,
        None,
        None,
    )
}
