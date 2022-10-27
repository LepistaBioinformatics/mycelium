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
    pub is_manager: bool,

    /// Superuser accounts allow their guest users to walking through specific
    /// accounts aiming to verify records irregularities and perform editions
    /// and deletions if necessary. Such users can perform mass deletions.
    pub is_staff: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDTO {
    pub id: Option<Uuid>,

    pub owner: ParentEnum<Uuid, UserDTO>,
    pub account_type: ParentEnum<Uuid, AccountTypeDTO>,
    pub guest_users: Option<ChildrenEnum<Uuid, GuestUserDTO>>,
}

impl AccountDTO {
    pub fn build_owner_url(&self, base_url: String) -> Result<String, ()> {
        match self.owner.to_owned() {
            ParentEnum::Id(id) => Ok(format!("{}/{}", base_url, id)),
            ParentEnum::Record(record) => match record.id {
                None => Ok(base_url),
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
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
                None => Ok(base_url),
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
            },
        }
    }

    pub fn build_guest_users_url(
        &self,
        base_url: String,
    ) -> Result<Vec<String>, ()> {
        match self.guest_users.to_owned() {
            None => Err(()),
            Some(records) => match records {
                ChildrenEnum::Ids(ids) => Ok(ids
                    .iter()
                    .map(|id| format!("{}/{}", base_url, id))
                    .collect()),
                ChildrenEnum::Records(records) => {
                    let urls = records
                        .iter()
                        .filter_map(|record| match record.id {
                            None => Some(base_url.to_owned()),
                            Some(_) => Some(format!(
                                "{}/{}",
                                base_url,
                                record.id.unwrap()
                            )),
                        })
                        .collect();

                    Ok(urls)
                }
            },
        }
    }
}

// ? --------------------------------------------------------------------------
// ? TESTS
// ? --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::email::EmailDTO;

    #[test]
    fn test_if_account_works() {
        let base_url = "http://local.host/api/v1/accounts".to_string();

        let account_type = AccountTypeDTO {
            id: None,
            name: "".to_string(),
            description: "".to_string(),
            is_manager: false,
            is_staff: false,
        };

        let user = UserDTO {
            id: None,
            username: "username".to_string(),
            email: EmailDTO::from_string("username@email.domain".to_string())
                .unwrap(),
            first_name: "first_name".to_string(),
            last_name: "last_name".to_string(),
        };

        let account = AccountDTO {
            id: None,
            owner: ParentEnum::Record(user),
            account_type: ParentEnum::Record(account_type),
            guest_users: None,
        };

        println!("{:?}", account.build_account_type_url(base_url.to_owned()));

        assert_eq!(
            account.build_account_type_url(base_url.to_owned()).is_ok(),
            true
        );

        assert_eq!(
            account.build_account_type_url(base_url.to_owned()).unwrap(),
            base_url.to_owned()
        );
    }
}
