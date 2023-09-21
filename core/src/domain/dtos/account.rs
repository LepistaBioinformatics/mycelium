use super::{guest::GuestUser, user::User};

use chrono::{DateTime, Local};
use clean_base::{
    dtos::{Children, Parent},
    utils::errors::{factories::invalid_arg_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountType {
    pub id: Option<Uuid>,

    pub name: String,
    pub description: String,

    pub is_subscription: bool,
    pub is_manager: bool,
    pub is_staff: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountTypeEnum {
    Standard,
    Manager,
    Staff,
    Subscription,
}

impl Display for AccountTypeEnum {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            AccountTypeEnum::Standard => write!(f, "Standard"),
            AccountTypeEnum::Manager => write!(f, "Manager"),
            AccountTypeEnum::Staff => write!(f, "Staff"),
            AccountTypeEnum::Subscription => write!(f, "Subscription"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum VerboseStatus {
    Pending,
    Active,
    Inactive,
    Archived,
    Unknown,
}

impl FromStr for VerboseStatus {
    type Err = VerboseStatus;

    fn from_str(s: &str) -> Result<VerboseStatus, VerboseStatus> {
        match s {
            "pending" => Ok(VerboseStatus::Pending),
            "active" => Ok(VerboseStatus::Active),
            "inactive" => Ok(VerboseStatus::Inactive),
            "archived" => Ok(VerboseStatus::Archived),
            _ => Err(VerboseStatus::Unknown),
        }
    }
}

impl Display for VerboseStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            VerboseStatus::Pending => write!(f, "pending"),
            VerboseStatus::Active => write!(f, "active"),
            VerboseStatus::Inactive => write!(f, "inactive"),
            VerboseStatus::Archived => write!(f, "archived"),
            VerboseStatus::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FlagResponse {
    pub is_active: Option<bool>,
    pub is_checked: Option<bool>,
    pub is_archived: Option<bool>,
}

impl VerboseStatus {
    pub fn from_flags(
        is_active: bool,
        is_checked: bool,
        is_archived: bool,
    ) -> Self {
        if is_active == false {
            return VerboseStatus::Inactive;
        }

        if is_checked == false {
            return VerboseStatus::Pending;
        }

        if is_archived == true {
            return VerboseStatus::Archived;
        }

        if is_archived == false {
            return VerboseStatus::Active;
        }

        VerboseStatus::Unknown
    }

    pub fn to_flags(&self) -> Result<FlagResponse, MappedErrors> {
        match self {
            VerboseStatus::Inactive => Ok(FlagResponse {
                is_active: Some(false),
                is_checked: None,
                is_archived: None,
            }),
            VerboseStatus::Pending => Ok(FlagResponse {
                is_active: Some(true),
                is_checked: Some(false),
                is_archived: None,
            }),
            VerboseStatus::Archived => Ok(FlagResponse {
                is_active: Some(true),
                is_checked: Some(true),
                is_archived: Some(true),
            }),
            VerboseStatus::Active => Ok(FlagResponse {
                is_active: Some(true),
                is_checked: Some(true),
                is_archived: Some(false),
            }),
            VerboseStatus::Unknown => invalid_arg_err(
                "Account status could not be `Unknown`".to_string(),
            )
            .as_error(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: Option<Uuid>,

    pub name: String,

    // Account statuses and verbose status
    //
    // Account statuses are used to determine the real (verbose) state of the
    // account.
    pub is_active: bool,
    pub is_checked: bool,
    pub is_archived: bool,
    pub verbose_status: Option<VerboseStatus>,

    // If current account is the default one
    //
    // Default account is the one that is created when the system is
    // initialized. Every user further created will be associated with this
    // account.
    pub is_default: bool,

    pub owners: Children<User, Uuid>,
    pub account_type: Parent<AccountType, Uuid>,
    pub guest_users: Option<Children<GuestUser, Uuid>>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}

impl Account {
    pub fn new(
        account_name: String,
        principal_owner: User,
        account_type: AccountType,
    ) -> Self {
        Self {
            id: None,
            name: account_name,
            is_active: true,
            is_checked: false,
            is_archived: false,
            verbose_status: None,
            is_default: false,
            owners: Children::Records([principal_owner].to_vec()),
            account_type: Parent::Record(account_type),
            guest_users: None,
            created: Local::now(),
            updated: None,
        }
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::email::Email;

    use chrono::Local;

    #[test]
    fn test_if_account_works() {
        let account_type = AccountType {
            id: None,
            name: "".to_string(),
            description: "".to_string(),
            is_subscription: false,
            is_manager: false,
            is_staff: false,
        };

        let account = Account {
            id: None,
            name: String::from("Account Name"),
            is_active: true,
            is_checked: false,
            is_archived: false,
            verbose_status: None,
            is_default: false,
            owners: Children::Records([].to_vec()),
            account_type: Parent::Record(account_type),
            guest_users: None,
            created: Local::now(),
            updated: Some(Local::now()),
        };

        User::new(
            None,
            "username".to_string(),
            Email::from_string("username@email.domain".to_string()).unwrap(),
            Some("first_name".to_string()),
            Some("last_name".to_string()),
            true,
            Local::now(),
            Some(Local::now()),
            Some(Parent::Record(account)),
        )
        .with_principal(false);
    }

    #[test]
    fn test_if_verbose_status_works() {
        [
            ((false, true, true), VerboseStatus::Inactive),
            ((false, false, true), VerboseStatus::Inactive),
            ((false, true, false), VerboseStatus::Inactive),
            ((false, false, false), VerboseStatus::Inactive),
            ((true, false, false), VerboseStatus::Pending),
            ((true, false, true), VerboseStatus::Pending),
            ((true, true, true), VerboseStatus::Archived),
            ((true, true, false), VerboseStatus::Active),
            // Unknown responses should not be returned over all above
            // combinations. Them, all will be tested.
            ((false, true, true), VerboseStatus::Unknown),
            ((false, false, true), VerboseStatus::Unknown),
            ((false, true, false), VerboseStatus::Unknown),
            ((false, false, false), VerboseStatus::Unknown),
            ((true, false, false), VerboseStatus::Unknown),
            ((true, false, true), VerboseStatus::Unknown),
            ((true, true, true), VerboseStatus::Unknown),
            ((true, true, false), VerboseStatus::Unknown),
        ]
        .into_iter()
        .for_each(|(flags, expected_value)| {
            let (is_active, is_checked, is_archived) = flags;

            let status =
                VerboseStatus::from_flags(is_active, is_checked, is_archived);

            // Unknown could not be returned from `from_flags` method
            if let VerboseStatus::Unknown = expected_value {
                assert_ne!(status, expected_value);
            } else {
                assert_eq!(status, expected_value);
            }

            let flags_response = status.to_flags().unwrap();

            match expected_value {
                VerboseStatus::Inactive => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap_or(is_checked),
                            flags_response.is_archived.unwrap_or(is_archived)
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Pending => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap(),
                            flags_response.is_archived.unwrap_or(is_archived)
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Archived => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap(),
                            flags_response.is_archived.unwrap()
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Active => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap(),
                            flags_response.is_archived.unwrap()
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Unknown => (),
            };
        });
    }
}
