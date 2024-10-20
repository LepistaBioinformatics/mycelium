use super::{
    account::VerboseStatus, guest::Permissions,
    related_accounts::RelatedAccounts, user::User,
};

use mycelium_base::utils::errors::{dto_err, execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LicensedResources {
    /// The guest account unique id
    ///
    /// This is the unique identifier of the account that is own of the
    /// resource to be managed.
    #[serde(alias = "guest_account_id")]
    pub acc_id: Uuid,

    /// If the guest account is the standard account
    ///
    /// Standard accounts has permissions to act as special users into the
    /// Mycelium system.
    #[serde(alias = "guest_account_is_default")]
    pub is_acc_std: bool,

    /// The guest account tenant unique id
    ///
    /// This is the unique identifier of the tenant that is own of the resource
    /// to be managed.
    pub tenant_id: Uuid,

    /// The guest account name
    ///
    /// This is the name of the account that is own of the resource to be
    /// managed.
    #[serde(alias = "guest_account_name")]
    pub acc_name: String,

    /// The guest account role unique id
    ///
    /// This is the unique identifier of the role that is own of the resource
    /// to be managed.
    pub guest_role_id: Uuid,

    /// The guest account role name
    ///
    /// This is the name of the role that is own of the resource to be
    /// managed.
    pub guest_role_name: String,

    /// The guest account role verbose name
    ///
    /// This is the verbose name of the role that is own of the resource to be
    /// managed.
    pub role: String,

    /// The guest role permissions
    ///
    /// This is the list of permissions that the guest role has.
    #[serde(alias = "permissions")]
    pub perms: Vec<Permissions>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    pub id: Uuid,

    /// The owner email
    ///
    /// The email of the user that administrate the profile. Email denotes the
    /// central part of the profile management. Email should be used to collect
    /// licensed IDs and perform guest operations. Thus, it should be unique in
    /// the Mycelium platform.
    pub email: String,

    /// The owner first name
    pub first_name: Option<String>,

    /// The owner last name
    pub last_name: Option<String>,

    /// The owner username
    pub username: Option<String>,
}

impl Owner {
    pub fn from_user(user: User) -> Result<Self, MappedErrors> {
        let user_id = match user.id {
            Some(id) => id,
            None => {
                return dto_err("User ID should not be empty".to_string())
                    .as_error()
            }
        };

        Ok(Self {
            id: user_id,
            email: user.email.get_email(),
            first_name: user.first_name,
            last_name: user.last_name,
            username: Some(user.username),
        })
    }
}

/// This object should be used over the application layer operations.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(alias = "owner_credentials")]
    pub owners: Vec<Owner>,

    /// The account unique id
    ///
    /// Such ID is related to the account primary-key instead of the owner
    /// primary key. In the case of the subscription accounts (accounts flagged
    /// with `is_subscription`) such ID should be propagated along the
    /// application flow.
    #[serde(alias = "current_account_id")]
    pub acc_id: Uuid,

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
    pub verbose_status: Option<VerboseStatus>,

    /// Accounts guested to the current profile
    ///
    /// Guest accounts delivers information about the guest account role and
    /// their respective permissions inside the host account. A single account
    /// should be several licenses into the same account.
    pub licensed_resources: Option<Vec<LicensedResources>>,
}

impl Profile {
    pub fn profile_string(&self) -> String {
        format!("profile/{}", self.acc_id.to_string())
    }

    pub fn get_owners_ids(&self) -> Vec<Uuid> {
        self.owners.iter().map(|i| i.id).collect()
    }

    pub fn has_admin_privileges(&self) -> bool {
        self.is_staff || self.is_manager
    }

    pub fn has_admin_privileges_or_error(&self) -> Result<(), MappedErrors> {
        match self.is_staff || self.is_manager {
            false => execution_err(
                "Current account has no administration privileges".to_string(),
            )
            .with_exp_true()
            .as_error(),
            true => Ok(()),
        }
    }

    /// Filter the licensed resources to the tenant
    ///
    /// This method should be used to filter licensed resources to the tenant
    /// that the profile is currently working on.
    pub fn on_tenant(&self, tenant_id: Uuid) -> Self {
        //
        // Filter the licensed resources to the tenant
        //
        let licensed_resources =
            if let Some(resources) = self.licensed_resources.as_ref() {
                let tenant_resources: Vec<LicensedResources> = resources
                    .iter()
                    .filter(|i| i.tenant_id == tenant_id)
                    .cloned()
                    .collect();

                if tenant_resources.is_empty() {
                    None
                } else {
                    Some(tenant_resources)
                }
            } else {
                None
            };

        //
        // Return the new profile
        //
        Self {
            licensed_resources,
            ..self.clone()
        }
    }

    // ? -----------------------------------------------------------------------
    // ? View filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with view permissions.
    pub fn get_view_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permissions::View, roles, None)
    }

    /// Filter IDs with view permissions with error if empty.
    pub fn get_view_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::View, roles, None)
    }

    /// Filter IDs with view permissions to accounts with error if empty.
    pub fn get_related_account_with_view_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::View,
            roles,
            None,
        )
    }

    /// Filter IDs with view permissions to default accounts with error if
    /// empty.
    pub fn get_default_view_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::View, roles, Some(true))
    }

    /// Filter RelatedAccounts with view permissions to default accounts with
    /// error if empty.
    pub fn get_related_account_with_default_view_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::View,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Create filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with create permissions.
    pub fn get_create_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permissions::Create, roles, None)
    }

    /// Filter IDs with create permissions with error if empty.
    pub fn get_create_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::Create, roles, None)
    }

    /// Filter IDs with create permissions to accounts with error if empty.
    pub fn get_related_account_with_create_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::Create,
            roles,
            None,
        )
    }

    /// Filter IDs with create permissions to default accounts with error if
    /// empty.
    pub fn get_default_create_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::Create, roles, Some(true))
    }

    /// Filter RelatedAccounts with create permissions to default accounts with
    /// error if empty.
    pub fn get_related_account_with_default_create_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::Create,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Update filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with update permissions.
    pub fn get_update_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permissions::Update, roles, None)
    }

    /// Filter IDs with update permissions with error if empty.
    pub fn get_update_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::Update, roles, None)
    }

    /// Filter IDs with update permissions to accounts with error if empty.
    pub fn get_related_account_with_update_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::Update,
            roles,
            None,
        )
    }

    /// Filter IDs with update permissions to default accounts with error if
    /// empty.
    pub fn get_default_update_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::Update, roles, Some(true))
    }

    /// Filter RelatedAccounts with update permissions to default accounts with
    /// error if empty.
    pub fn get_related_account_with_default_update_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::Update,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Delete filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with delete permissions.
    pub fn get_delete_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permissions::Delete, roles, None)
    }

    /// Filter IDs with delete permissions with error if empty.
    pub fn get_delete_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::Delete, roles, None)
    }

    /// Filter IDs with delete permissions to accounts with error if empty.
    pub fn get_related_account_with_delete_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::Delete,
            roles,
            None,
        )
    }

    /// Filter IDs with delete permissions to default accounts with error if
    /// empty.
    pub fn get_default_delete_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permissions::Delete, roles, Some(true))
    }

    /// Filter RelatedAccounts with delete permissions to default accounts with
    /// error if empty.
    pub fn get_related_account_with_default_delete_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permissions::Delete,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Basic filter functions
    // ? -----------------------------------------------------------------------

    /// Create a list of licensed ids.
    ///
    /// Licensed ids are Uuids of accounts which the current profile has access
    /// to do based on the specified `PermissionsType`.
    fn get_licensed_ids<T: ToString>(
        &self,
        permission: Permissions,
        roles: Vec<T>,
        should_be_default: Option<bool>,
    ) -> Vec<Uuid> {
        if let None = self.licensed_resources {
            return vec![self.acc_id];
        }

        let licensed_resources = if let Some(true) = should_be_default {
            self.licensed_resources
                .as_ref()
                .unwrap()
                .into_iter()
                .filter_map(|license| match license.is_acc_std {
                    true => Some(license.to_owned()),
                    false => None,
                })
                .collect::<Vec<LicensedResources>>()
        } else {
            self.licensed_resources.as_ref().unwrap().to_vec()
        };

        licensed_resources
            .into_iter()
            .filter_map(|i| {
                match i.perms.contains(&permission)
                    && roles
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<String>>()
                        .contains(&i.role)
                {
                    false => None,
                    true => Some(i.acc_id),
                }
            })
            .collect::<Vec<Uuid>>()
    }

    fn get_licensed_ids_or_error<T: ToString>(
        &self,
        permission: Permissions,
        roles: Vec<T>,
        should_be_default: Option<bool>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        let ids = self.get_licensed_ids(permission, roles, should_be_default);

        if !vec![!ids.is_empty(), self.is_staff, self.is_manager]
            .into_iter()
            .any(|i| i == true)
        {
            return execution_err(
                "Insufficient privileges to perform these action".to_string(),
            )
            .with_exp_true()
            .as_error();
        }

        Ok(ids)
    }

    /// Check if the current profile has admin privileges or the licensed ids
    /// are not empty. If so, return the licensed ids. Otherwise, return an
    /// error.
    ///
    /// The Staff related account has high priority over the manager related
    /// account. If the current profile is a staff, the function should return
    /// `HasStaffPrivileges`. If the current profile is a manager, the function
    /// should return `HasManagerPrivileges`. If the current profile has no
    /// admin privileges and the licensed ids are empty, the function should
    /// return an error.
    ///
    fn get_licensed_ids_as_related_accounts_or_error<T: ToString>(
        &self,
        permission: Permissions,
        roles: Vec<T>,
        should_be_default: Option<bool>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        if self.is_staff {
            return Ok(RelatedAccounts::HasStaffPrivileges);
        }

        if self.is_manager {
            return Ok(RelatedAccounts::HasManagerPrivileges);
        }

        let ids = self.get_licensed_ids(permission, roles, should_be_default);

        if ids.is_empty() {
            return execution_err(
                "Insufficient privileges to perform these action".to_string(),
            )
            .with_exp_true()
            .as_error();
        }

        Ok(RelatedAccounts::AllowedAccounts(ids))
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{LicensedResources, Owner, Profile};
    use crate::domain::dtos::guest::Permissions;
    use std::str::FromStr;
    use test_log::test;
    use uuid::Uuid;

    #[test]
    fn profile_get_ids_works() {
        let profile = Profile {
            owners: vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "agrobiota-results-expert-creator@biotrop.com.br"
                    .to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
            }],
            acc_id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
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
                    acc_id: Uuid::from_str(
                        "e497848f-a0d4-49f4-8288-c3df11416ff1",
                    )
                    .unwrap(),
                    tenant_id: Uuid::from_str(
                        "e497848f-a0d4-49f4-8288-c3df11416ff1",
                    )
                    .unwrap(),
                    acc_name: "guest_account_name".to_string(),
                    is_acc_std: false,
                    guest_role_id: Uuid::from_str(
                        "e497848f-a0d4-49f4-8288-c3df11416ff2",
                    )
                    .unwrap(),
                    guest_role_name: "guest_role_name".to_string(),
                    role: "service".to_string(),
                    perms: [Permissions::View, Permissions::Create].to_vec(),
                }]
                .to_vec(),
            ),
        };

        let ids = profile.get_create_ids(["service".to_string()].to_vec());

        assert!(ids.len() == 1);
    }

    #[test]
    fn get_licensed_ids_or_error_works() {
        let desired_role = "service".to_string();

        let mut profile = Profile {
            owners: vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "agrobiota-results-expert-creator@biotrop.com.br"
                    .to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
            }],
            acc_id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
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
                    acc_id: Uuid::from_str(
                        "e497848f-a0d4-49f4-8288-c3df11416ff1",
                    )
                    .unwrap(),
                    tenant_id: Uuid::from_str(
                        "e497848f-a0d4-49f4-8288-c3df11416ff1",
                    )
                    .unwrap(),
                    acc_name: "guest_account_name".to_string(),
                    is_acc_std: false,
                    guest_role_id: Uuid::from_str(
                        "e497848f-a0d4-49f4-8288-c3df11416ff2",
                    )
                    .unwrap(),
                    guest_role_name: "guest_role_name".to_string(),
                    role: desired_role.to_owned(),
                    perms: [Permissions::View, Permissions::Create].to_vec(),
                }]
                .to_vec(),
            ),
        };

        assert_eq!(
            false,
            profile
                .get_update_ids_or_error([desired_role.to_owned()].to_vec(),)
                .is_ok(),
        );

        assert_eq!(
            false,
            profile
                .get_update_ids_or_error([desired_role.to_owned()].to_vec(),)
                .is_ok(),
        );

        profile.is_manager = true;

        assert_eq!(
            true,
            profile
                .get_update_ids_or_error([desired_role.to_owned()].to_vec(),)
                .is_ok(),
        );

        profile.is_manager = false;
        profile.is_staff = true;

        assert_eq!(
            true,
            profile
                .get_update_ids_or_error([desired_role.to_owned()].to_vec(),)
                .is_ok(),
        );

        profile.is_staff = false;

        assert_eq!(
            false,
            profile
                .get_update_ids_or_error([desired_role].to_vec())
                .is_ok(),
        );
    }
}
