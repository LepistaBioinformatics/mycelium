use super::{account::VerboseProfileStatus, guest::PermissionsType};

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
    /// The owner email
    ///
    /// The email of the user that administrate the profile. Email denotes the
    /// central part of the profile management. Email should be used to collect
    /// licensed IDs and perform guest operations. Thus, it should be unique in
    /// the Mycelium platform.
    pub email: String,

    /// The account unique id
    ///
    /// Such ID is related to the account primary-key instead of the owner
    /// primary key. In the case of the subscription accounts (accounts flagged
    /// with `is_subscription`) such ID should be propagated along the
    /// application flow.
    pub current_account_id: Uuid,

    /// If profile belongs to a `subscription` account
    ///
    /// Subscription accounts should be used to manage legal entities. Only
    /// subscription accounts should receive guest accounts.
    pub is_subscription: bool,

    /// If profile belongs to a `manager` account
    ///
    /// Manager accounts should be used by users with elevated privileges inside
    /// the Mycelium platform. Such user should perform actions like create
    /// roles, guest-roles, guest default-user accounts to work into
    /// subscription accounts.
    pub is_manager: bool,

    /// If profile belongs to a `staff` account
    ///
    /// Staff user has elevated roles into the application. Like managers, staff
    /// users has elevated privileges. Only staff user has permission to
    /// delegate other staffs.
    pub is_staff: bool,

    /// If the account owner is active
    ///
    /// Profiles exists to abstract account privileges. If the profile is
    /// related to an inactive owner the profile could not perform any action.
    /// Only staff or manager user should perform the activation of such users.
    pub owner_is_active: bool,

    /// If the account itself is inactive
    ///
    /// When inactive accounts should not perform internal operations.
    pub account_is_active: bool,

    /// If the account was approved after registration
    ///
    /// New accounts should be approved by manager or staff users after their
    /// registration into the Mycelium platform. Case the approval was
    /// performed, this flag should be true.
    pub account_was_approved: bool,

    /// If the account was archived after registration
    ///
    /// New accounts should be archived. After archived accounts should not be
    /// included at default filtering actions.
    pub account_was_archived: bool,

    /// Indicate the profile status for humans
    ///
    /// The profile status is composed of all account flags statuses
    /// composition. But it is not readable for humans. These struct attribute
    /// allows human users to understand the account status without read the
    /// flags, avoiding misinterpretation of this.
    pub verbose_status: Option<VerboseProfileStatus>,

    /// Accounts guested to the current profile
    ///
    /// Guest accounts delivers information about the guest account role and
    /// their respective permissions inside the host account. A single account
    /// should be several licenses into the same account.
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
                        match i.permissions.contains(&permission) &&
                            roles.contains(&i.role)
                        {
                            false => None,
                            true => Some(i.guest_account_id),
                        }
                    })
                    .collect::<Vec<Uuid>>();

                if include_itself.unwrap_or(false) {
                    println!("ids 2: {:?}", ids);
                    ids.append(&mut vec![self.current_account_id]);
                }

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
            owner_is_active: true,
            account_is_active: true,
            account_was_approved: true,
            account_was_archived: false,
            verbose_status: None,
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

        let ids =
            profile.get_create_ids(["service".to_string()].to_vec(), None);

        assert!(ids.len() == 1);
    }
}
