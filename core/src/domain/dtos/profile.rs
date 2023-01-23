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
                        match i.permissions.contains(&permission) &&
                            roles.contains(&i.role)
                        {
                            false => None,
                            true => Some(i.guest_account_id),
                        }
                    })
                    .collect::<Vec<Uuid>>();

                if include_itself.unwrap_or(false) {
                    ids.append(&mut vec![self.current_account_id]);
                }

                ids
            }
        }
    }
}
