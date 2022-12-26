use super::guest::PermissionsType;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicensedResourcesDTO {
    /// This is the unique identifier of the account that is own of the
    /// resource to be managed.
    pub guest_account_id: Uuid,
    pub role: String,
    pub permissions: Vec<PermissionsType>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}

/// This object should be used over the application layer operations.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDTO {
    pub email: String,
    pub current_account_id: Uuid,
    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,

    /// If the licensed IDs are `None`, the user has only permissions to act
    /// inside their own account.
    pub licensed_resources: Option<Vec<LicensedResourcesDTO>>,
}

impl ProfileDTO {
    /// Filter IDs with view permissions.
    pub fn get_view_ids(&self, roles: Vec<String>) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::View, roles)
    }

    /// Filter IDs with create permissions.
    pub fn get_create_ids(&self, roles: Vec<String>) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::Create, roles)
    }

    /// Filter IDs with update permissions.
    pub fn get_update_ids(&self, roles: Vec<String>) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::Update, roles)
    }

    /// Filter IDs with delete permissions.
    pub fn get_delete_ids(&self, roles: Vec<String>) -> Vec<Uuid> {
        self.get_licensed_ids(PermissionsType::Delete, roles)
    }

    /// Create a list of licensed ids.
    ///
    /// Licensed ids are Uuids of accounts which the current profile has access
    /// to do based on the specified `PermissionsType`.
    fn get_licensed_ids(
        &self,
        permission: PermissionsType,
        roles: Vec<String>,
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

                ids.append(&mut vec![self.current_account_id]);

                ids
            }
        }
    }
}
