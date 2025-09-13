mod meta;
mod verbose_status;

pub use meta::AccountMetaKey;
pub use verbose_status::{FlagResponse, VerboseStatus};

use super::{
    account_type::AccountType, guest_user::GuestUser, tag::Tag, user::User,
};
use crate::domain::{actors::SystemActor, dtos::written_by::WrittenBy};

use chrono::{DateTime, Local};
use mycelium_base::dtos::Children;
use serde::{Deserialize, Serialize};
use slugify::slugify;
use std::collections::HashMap;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

pub type AccountMeta = HashMap<AccountMetaKey, String>;

#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// The Account ID
    pub id: Option<Uuid>,

    /// The Account Name
    pub name: String,

    /// The Account Slug
    ///
    /// This is generated from the account name. This is used for programmatic
    /// access and verification of the account.
    ///
    pub slug: String,

    /// Account Tags
    ///
    /// Information about the account. This is used for categorizing and filter
    /// account.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,

    /// Account is active
    ///
    /// If the account is active. This is used for logic trash and restore
    /// account.
    ///
    pub is_active: bool,

    /// Account is checked
    ///
    /// If the account was verified by a human. This is used for account
    /// verification.
    ///
    pub is_checked: bool,

    /// Account is archived
    ///
    /// If the account is archived. This is used for account archiving.
    ///
    pub is_archived: bool,

    /// Account is deleted
    ///
    /// If the account is deleted. This is used for logic trash and restore
    /// account.
    ///
    pub is_deleted: bool,

    /// Verbose status
    ///
    /// Is the human readable status of the account.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose_status: Option<VerboseStatus>,

    // If current account is the default one
    //
    // Default account is the one that is created when the system is
    // initialized. Every user further created will be associated with this
    // account.
    pub is_system_account: bool,

    /// The Account Owners
    ///
    /// This is the list of account owners. The account owners are the users who
    /// have the account owner role.
    pub owners: Children<User, Uuid>,

    /// The Account Type
    ///
    /// Account type is the type of the account. The account type is used to
    /// categorize the account.
    pub account_type: AccountType,

    /// The Account Guest Users
    ///
    /// This is the list of guest users of the account.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guest_users: Option<Children<GuestUser, Uuid>>,

    /// The Account Created Date
    #[serde(alias = "created")]
    pub created_at: DateTime<Local>,

    /// The Account Created By
    ///
    /// The ID of the account that created the account. This is used for
    /// auditing purposes.
    ///
    pub created_by: Option<WrittenBy>,

    /// The Account Updated Date
    #[serde(alias = "updated", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Local>>,

    /// The Account Updated By
    ///
    /// The ID of the account that updated the account. This is used for
    /// auditing purposes.
    ///
    pub updated_by: Option<WrittenBy>,

    /// The Account Meta
    ///
    /// Store metadata about the account.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<HashMap<AccountMetaKey, String>>,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            id: None,
            name: String::new(),
            slug: String::new(),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            is_deleted: false,
            verbose_status: None,
            is_system_account: false,
            owners: Children::Ids([].to_vec()),
            account_type: AccountType::User,
            guest_users: None,
            created_at: Local::now(),
            created_by: None,
            updated_at: None,
            updated_by: None,
            meta: None,
        }
    }
}

impl Account {
    /// Create a new subscription account
    ///
    /// Use this method to create standard subscription accounts.
    pub fn new_subscription_account(
        account_name: String,
        tenant_id: Uuid,
        created_by: Option<WrittenBy>,
    ) -> Self {
        Self {
            id: None,
            name: account_name.to_owned(),
            slug: slugify!(account_name.as_str()),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            is_deleted: false,
            verbose_status: None,
            is_system_account: false,
            owners: Children::Ids([].to_vec()),
            account_type: AccountType::Subscription { tenant_id },
            guest_users: None,
            created_at: Local::now(),
            created_by,
            updated_at: None,
            updated_by: None,
            meta: None,
        }
    }

    pub fn new_role_related_account<T: ToString>(
        account_name: String,
        tenant_id: Uuid,
        read_role_id: Uuid,
        write_role_id: Uuid,
        role_name: T,
        is_system_account: bool,
        created_by: Option<WrittenBy>,
    ) -> Self {
        Self {
            id: None,
            name: account_name.to_owned(),
            slug: slugify!(account_name.as_str()),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            is_deleted: false,
            verbose_status: None,
            is_system_account,
            owners: Children::Ids([].to_vec()),
            account_type: AccountType::RoleAssociated {
                tenant_id,
                role_name: role_name.to_string(),
                read_role_id,
                write_role_id,
            },
            guest_users: None,
            created_at: Local::now(),
            created_by,
            updated_at: None,
            updated_by: None,
            meta: None,
        }
    }

    pub fn new_actor_related_account(
        name: String,
        actor: SystemActor,
        is_system_account: bool,
        created_by: Option<WrittenBy>,
    ) -> Self {
        Self {
            id: None,
            name: name.to_owned(),
            slug: slugify!(name.as_str()),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            is_deleted: false,
            verbose_status: None,
            is_system_account,
            owners: Children::Ids([].to_vec()),
            account_type: AccountType::ActorAssociated { actor },
            guest_users: None,
            created_at: Local::now(),
            created_by,
            updated_at: None,
            updated_by: None,
            meta: None,
        }
    }

    pub fn new_tenant_management_account(
        account_name: String,
        tenant_id: Uuid,
        created_by: Option<WrittenBy>,
    ) -> Self {
        Self {
            id: None,
            name: account_name.to_owned(),
            slug: slugify!(account_name.as_str()),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            is_deleted: false,
            verbose_status: None,
            is_system_account: false,
            owners: Children::Ids([].to_vec()),
            account_type: AccountType::TenantManager { tenant_id },
            guest_users: None,
            created_at: Local::now(),
            created_by,
            updated_at: None,
            updated_by: None,
            meta: None,
        }
    }

    pub fn with_id(&mut self) -> Self {
        self.id = Some(Uuid::new_v4());
        self.clone()
    }

    pub fn new(
        account_name: String,
        principal_owner: User,
        account_type: AccountType,
        created_by: Option<WrittenBy>,
    ) -> Self {
        Self {
            id: None,
            name: account_name.to_owned(),
            slug: slugify!(account_name.as_str()),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            is_deleted: false,
            verbose_status: None,
            is_system_account: false,
            owners: Children::Records([principal_owner].to_vec()),
            account_type,
            guest_users: None,
            created_at: Local::now(),
            created_by,
            updated_at: None,
            updated_by: None,
            meta: None,
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
    use mycelium_base::dtos::Parent;

    #[test]
    fn test_if_account_works() {
        let account = Account {
            id: None,
            name: String::from("Account Name"),
            slug: String::from("account-name"),
            tags: None,
            is_active: true,
            is_checked: false,
            is_archived: false,
            is_deleted: false,
            verbose_status: None,
            is_system_account: false,
            owners: Children::Records([].to_vec()),
            account_type: AccountType::User,
            guest_users: None,
            created_at: Local::now(),
            created_by: None,
            updated_at: Some(Local::now()),
            updated_by: None,
            meta: None,
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
            None,
        )
        .with_principal(false);
    }

    #[test]
    fn test_if_verbose_status_works() {
        [
            ((false, true, true, false), VerboseStatus::Inactive),
            ((false, false, true, false), VerboseStatus::Inactive),
            ((false, true, false, false), VerboseStatus::Inactive),
            ((false, false, false, false), VerboseStatus::Inactive),
            ((true, false, false, false), VerboseStatus::Unverified),
            ((true, false, true, false), VerboseStatus::Unverified),
            ((true, true, true, false), VerboseStatus::Archived),
            ((true, true, false, false), VerboseStatus::Verified),
            ((true, true, true, true), VerboseStatus::Deleted),
            // Unknown responses should not be returned over all above
            // combinations. Them, all will be tested.
            ((false, true, true, false), VerboseStatus::Unknown),
            ((false, false, true, false), VerboseStatus::Unknown),
            ((false, true, false, false), VerboseStatus::Unknown),
            ((false, false, false, false), VerboseStatus::Unknown),
            ((true, false, false, false), VerboseStatus::Unknown),
            ((true, false, true, false), VerboseStatus::Unknown),
            ((true, true, true, false), VerboseStatus::Unknown),
            ((true, true, false, false), VerboseStatus::Unknown),
        ]
        .into_iter()
        .for_each(|(flags, expected_value)| {
            let (is_active, is_checked, is_archived, is_deleted) = flags;

            let status = VerboseStatus::from_flags(
                is_active,
                is_checked,
                is_archived,
                is_deleted,
            );

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
                            flags_response.is_archived.unwrap_or(is_archived),
                            flags_response.is_deleted.unwrap_or(is_deleted)
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Unverified => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap(),
                            flags_response.is_archived.unwrap_or(is_archived),
                            flags_response.is_deleted.unwrap_or(is_deleted)
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Archived => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap(),
                            flags_response.is_archived.unwrap(),
                            flags_response.is_deleted.unwrap_or(is_deleted)
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Verified => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap(),
                            flags_response.is_archived.unwrap(),
                            flags_response.is_deleted.unwrap_or(is_deleted)
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Deleted => {
                    assert_eq!(
                        VerboseStatus::from_flags(
                            flags_response.is_active.unwrap(),
                            flags_response.is_checked.unwrap(),
                            flags_response.is_archived.unwrap_or(is_archived),
                            flags_response.is_deleted.unwrap(),
                        ),
                        expected_value
                    );
                }
                VerboseStatus::Unknown => (),
            };
        });
    }
}
