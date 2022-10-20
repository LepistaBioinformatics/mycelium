use super::{
    enums::{ChildrenEnum, ParentEnum},
    guest::GuestUserDTO,
    user::UserDTO,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTypeDTO {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: String,

    /// Manager accounts allow their guest users to walking through specific
    /// accounts aiming to verify records irregularities and perform editions
    /// and deletions if necessary.
    pub is_manager: Option<bool>,

    /// Superuser accounts allow their guest users to walking through specific
    /// accounts aiming to verify records irregularities and perform editions
    /// and deletions if necessary. Such users can perform mass deletions.
    pub is_staff: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDTO {
    pub id: Option<Uuid>,

    pub owner: ParentEnum<Uuid, UserDTO>,
    pub account_type: ParentEnum<Uuid, AccountTypeDTO>,
    pub guest_users: ChildrenEnum<Uuid, GuestUserDTO>,
}

impl AccountDTO {
    pub fn build_owner_url(&self, base_url: String) -> Result<String, ()> {
        match self.owner.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{}/{}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }

    pub fn build_account_type_url(
        &self,
        base_url: String,
    ) -> Result<String, ()> {
        match self.account_type.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{}/{}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }

    pub fn build_guest_users_url(
        &self,
        base_url: String,
    ) -> Result<Vec<String>, ()> {
        match self.guest_users.to_owned() {
            ChildrenEnum::Ids(ids) => Ok(ids
                .iter()
                .map(|id| format!("{}/{}", base_url, id))
                .collect()),
            ChildrenEnum::Records(records) => {
                let urls = records
                    .iter()
                    .filter_map(|record| match record.id {
                        None => None,
                        Some(_) => {
                            Some(format!("{}/{}", base_url, record.id.unwrap()))
                        }
                    })
                    .collect();

                Ok(urls)
            }
        }
    }
}
