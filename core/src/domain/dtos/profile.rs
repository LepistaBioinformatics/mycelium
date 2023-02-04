use super::guest::PermissionsType;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicensedResources {
    /// This is the unique identifier of the account that is own of the
    /// resource to be managed.
    pub guest_account_id: Uuid,
    pub role: String,
    pub permissions: Vec<PermissionsType>,
}

/// This object should be used over the application layer operations.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub email: String,
    pub current_account_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,

    /// If the licensed IDs are `None`, the user has only permissions to act
    /// inside their own account.
    pub licensed_resources: Option<Vec<LicensedResources>>,
}

impl Profile {
    /// Filter IDs with view permissions.
    pub fn get_view_ids(
        &self,
        roles: Vec<String>,
        include_itself: Option<bool>,
    ) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::View, roles, include_itself)
    }

    /// Filter IDs with create permissions.
    pub fn get_create_ids(
        &self,
        roles: Vec<String>,
        include_itself: Option<bool>,
    ) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::Create, roles, include_itself)
    }

    /// Filter IDs with update permissions.
    pub fn get_update_ids(
        &self,
        roles: Vec<String>,
        include_itself: Option<bool>,
    ) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::Update, roles, include_itself)
    }

    /// Filter IDs with delete permissions.
    pub fn get_delete_ids(
        &self,
        roles: Vec<String>,
        include_itself: Option<bool>,
    ) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::Delete, roles, include_itself)
    }

    /// Create a list of licensed ids.
    ///
    /// Licensed ids are Uuids of accounts which the current profile has access
    /// to do based on the specified `PermissionsType`.
    fn get_licensed_ids(
        &self,
        permission: PermissionsType,
        roles: Vec<String>,
        include_itself: Option<bool>,
    ) -> Vec<Uuid> {
        match &self.licensed_resources {
            None => vec![self.current_account_id],
            Some(res) => {
                let mut ids = res
                    .into_iter()
                    .filter_map(|i| {
                        println!("i: {:?}", i);
                        println!("permission: {:?}", permission);
                        println!("roles: {:?}", roles);

                        match i.permissions.contains(&permission) &&
                            roles.contains(&i.role)
                        {
                            false => None,
                            true => Some(i.guest_account_id),
                        }
                    })
                    .collect::<Vec<Uuid>>();

                println!("ids 1: {:?}", ids);

                if include_itself.unwrap_or(false) {
                    println!("ids 2: {:?}", ids);
                    ids.append(&mut vec![self.current_account_id]);
                }

                println!("ids 3: {:?}", ids);

                ids
            }
        }
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{LicensedResources, Profile};
    use crate::domain::dtos::guest::PermissionsType;
    use std::str::FromStr;
    use test_log::test;
    use uuid::Uuid;

    #[test]
    fn profile_get_ids_works() {
        let profile = Profile {
            email: "agrobiota-results-expert-creator@biotrop.com.br"
                .to_string(),
            current_account_id: Uuid::from_str(
                "d776e96f-9417-4520-b2a9-9298136031b0",
            )
            .unwrap(),
            is_subscription: false,
            is_manager: false,
            is_staff: false,
            licensed_resources: Some(
                [LicensedResources {
                    guest_account_id: Uuid::from_str(
                        "e497848f-a0d4-49f4-8288-c3df11416ff1",
                    )
                    .unwrap(),
                    role: "service".to_string(),
                    permissions: [
                        PermissionsType::View,
                        PermissionsType::Create,
                    ]
                    .to_vec(),
                }]
                .to_vec(),
            ),
        };

        println!("profile: {:?}", profile);

        let ids =
            profile.get_create_ids(["service".to_string()].to_vec(), None);

        println!("ids: {:?}", ids);
    }
}
