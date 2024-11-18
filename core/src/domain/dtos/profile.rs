use std::str::FromStr;

use super::{
    account::VerboseStatus, guest_role::Permission,
    native_error_codes::NativeErrorCodes, related_accounts::RelatedAccounts,
    user::User,
};

use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{dto_err, execution_err, MappedErrors};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LicensedResource {
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
    #[serde(alias = "permission")]
    pub perm: Permission,
}

impl LicensedResource {
    fn is_uuid(value: &str) -> bool {
        let uuid_format = vec![8, 4, 4, 4, 12];
        let mut chars = value.chars().peekable();

        for &count in &uuid_format {
            for _ in 0..count {
                match chars.next() {
                    Some(c) if c.is_ascii_hexdigit() => continue,
                    _ => return false,
                }
            }

            if let Some('-') = chars.peek() {
                chars.next();
            }
        }
        chars.next().is_none()
    }

    /// Try to load a UUID v4 hex as UUID from a string
    ///
    pub fn load_uuid(value: String) -> Result<Uuid, MappedErrors> {
        match Uuid::from_str(&value) {
            Ok(uuid) => Ok(uuid),
            Err(_) => execution_err(format!("Invalid UUID: {}", value))
                .with_code(NativeErrorCodes::MYC00019)
                .with_exp_true()
                .as_error(),
        }
    }
}

impl ToString for LicensedResource {
    fn to_string(&self) -> String {
        //
        // Encode account name as base64
        //
        let encoded_account_name =
            general_purpose::STANDARD.encode(self.acc_name.as_bytes());

        format!(
            "tid/{tenant_id}/aid/{acc_id}/gid/{guest_role_id}?pr={role}:{perm}&std={is_acc_std}&name={acc_name}",
            tenant_id = self.tenant_id.to_string().replace("-", ""),
            acc_id = self.acc_id.to_string().replace("-", ""),
            guest_role_id = self.guest_role_id.to_string().replace("-", ""),
            role = self.role,
            perm = self.perm.to_owned().to_i32(),
            is_acc_std = self.is_acc_std as i8,
            acc_name = encoded_account_name,
        )
    }
}

impl FromStr for LicensedResource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let full_url = format!("https://localhost.local/{s}");

        let url = Url::from_str(&full_url).map_err(|e| {
            format!("Unexpected error on check license URL: {:?}", e)
        })?;

        //
        // Extract the path segments
        //
        let segments: Vec<_> =
            url.path_segments().ok_or("Path not found")?.collect();

        if segments.len() != 6
            || segments[0] != "tid"
            || segments[2] != "aid"
            || segments[4] != "gid"
        {
            return Err("Invalid path format".to_string());
        }

        let tenant_id = segments[1];
        let account_id = segments[3];
        let guest_role_id = segments[5];

        if !Self::is_uuid(tenant_id) {
            return Err("Invalid tenant UUID".to_string());
        }

        if !Self::is_uuid(account_id) {
            return Err("Invalid account UUID".to_string());
        }

        if !Self::is_uuid(guest_role_id) {
            return Err("Invalid guest role UUID".to_string());
        }

        //
        // Extract the query parameters
        //
        let permissioned_role = url
            .query_pairs()
            .find(|(key, _)| key == "pr")
            .map(|(_, value)| value)
            .ok_or("Parameter pr not found")?;

        let permissioned_role: Vec<_> = permissioned_role.split(':').collect();

        if permissioned_role.len() != 2 {
            return Err("Invalid permissioned role format".to_string());
        }

        let role_name = permissioned_role[0];
        let permission_code = permissioned_role[1];

        let std = match url
            .query_pairs()
            .find(|(key, _)| key == "std")
            .map(|(_, value)| value)
            .ok_or("Parameter std not found")?
            .parse::<i8>()
        {
            Ok(std) => match std {
                0 => false,
                1 => true,
                _ => {
                    return Err("Invalid account standard".to_string());
                }
            },
            Err(_) => {
                return Err("Failed to parse account standard".to_string());
            }
        };

        let name_encoded = url
            .query_pairs()
            .find(|(key, _)| key == "name")
            .map(|(_, value)| value)
            .ok_or("Parameter name not found")?;

        let name_decoded =
            match general_purpose::STANDARD.decode(name_encoded.as_bytes()) {
                Ok(name) => name,
                Err(_) => {
                    return Err("Failed to decode account name".to_string());
                }
            };

        Ok(Self {
            tenant_id: Uuid::from_str(tenant_id).unwrap(),
            acc_id: Uuid::from_str(account_id).unwrap(),
            role: role_name.to_string(),
            perm: Permission::from_i32(permission_code.parse::<i32>().unwrap()),
            is_acc_std: std,
            acc_name: String::from_utf8(name_decoded).unwrap(),
            guest_role_id: guest_role_id.to_string().parse::<Uuid>().unwrap(),
            guest_role_name: "guest_role_name".to_string(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LicensedResources {
    Records(Vec<LicensedResource>),
    Urls(Vec<String>),
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

    /// If the owner is the principal account owner
    pub is_principal: bool,
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
            first_name: user.to_owned().first_name,
            last_name: user.to_owned().last_name,
            username: Some(user.to_owned().username),
            is_principal: user.is_principal(),
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
    pub licensed_resources: Option<LicensedResources>,
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
            .with_code(NativeErrorCodes::MYC00019)
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
                let records: Vec<LicensedResource> = match resources {
                    LicensedResources::Records(records) => records
                        .iter()
                        .filter(|i| i.tenant_id == tenant_id)
                        .map(|i| i.to_owned())
                        .collect(),
                    LicensedResources::Urls(urls) => urls
                        .iter()
                        .map(|i| LicensedResource::from_str(i).unwrap())
                        .filter(|i| i.tenant_id == tenant_id)
                        .map(|i| i.to_owned())
                        .collect(),
                };

                if records.is_empty() {
                    None
                } else {
                    Some(LicensedResources::Records(records))
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
    // ? Read filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with read permissions.
    pub fn get_read_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::Read, roles, None)
    }

    /// Filter IDs with read permissions with error if empty.
    pub fn get_read_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Read, roles, None)
    }

    /// Filter IDs with read permissions to accounts with error if empty.
    pub fn get_related_account_with_read_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Read,
            roles,
            None,
        )
    }

    /// Filter IDs with read permissions to default accounts with error if
    /// empty.
    pub fn get_default_read_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Read, roles, Some(true))
    }

    /// Filter RelatedAccounts with read permissions to default accounts with
    /// error if empty.
    pub fn get_related_account_with_default_read_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Read,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Write filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with write permissions.
    pub fn get_write_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::Write, roles, None)
    }

    /// Filter IDs with write permissions with error if empty.
    pub fn get_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, None)
    }

    /// Filter IDs with write permissions to accounts with error if empty.
    pub fn get_related_account_with_write_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Write,
            roles,
            None,
        )
    }

    /// Filter IDs with write permissions to default accounts with error if
    /// empty.
    pub fn get_default_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, Some(true))
    }

    /// Filter RelatedAccounts with write permissions to default accounts with
    /// error if empty.
    pub fn get_related_account_with_default_write_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Write,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Read/Write filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with write permissions.
    pub fn get_read_write_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::ReadWrite, roles, None)
    }

    /// Filter IDs with write permissions with error if empty.
    pub fn get_read_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::ReadWrite, roles, None)
    }

    /// Filter IDs with write permissions to accounts with error if empty.
    pub fn get_related_account_with_read_write_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::ReadWrite,
            roles,
            None,
        )
    }

    /// Filter IDs with write permissions to default accounts with error if
    /// empty.
    pub fn get_default_read_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::ReadWrite, roles, Some(true))
    }

    /// Filter RelatedAccounts with write permissions to default accounts with
    /// error if empty.
    pub fn get_related_account_with_default_read_write_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::ReadWrite,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Update filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with update permissions.
    #[deprecated(note = "Use get_write_ids instead")]
    pub fn get_update_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::Write, roles, None)
    }

    /// Filter IDs with update permissions with error if empty.
    #[deprecated(note = "Use get_write_ids_or_error instead")]
    pub fn get_update_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, None)
    }

    /// Filter IDs with update permissions to accounts with error if empty.
    #[deprecated(note = "Use get_related_account_with_write_or_error instead")]
    pub fn get_related_account_with_update_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Write,
            roles,
            None,
        )
    }

    /// Filter IDs with update permissions to default accounts with error if
    /// empty.
    #[deprecated(note = "Use get_default_write_ids_or_error instead")]
    pub fn get_default_update_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, Some(true))
    }

    /// Filter RelatedAccounts with update permissions to default accounts with
    /// error if empty.
    #[deprecated(
        note = "Use get_related_account_with_default_write_or_error instead"
    )]
    pub fn get_related_account_with_default_update_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Write,
            roles,
            Some(true),
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Delete filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with delete permissions.
    #[deprecated(note = "Use get_write_ids instead")]
    pub fn get_delete_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::Write, roles, None)
    }

    /// Filter IDs with delete permissions with error if empty.
    #[deprecated(note = "Use get_write_ids_or_error instead")]
    pub fn get_delete_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, None)
    }

    /// Filter IDs with delete permissions to accounts with error if empty.
    #[deprecated(note = "Use get_related_account_with_write_or_error instead")]
    pub fn get_related_account_with_delete_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Write,
            roles,
            None,
        )
    }

    /// Filter IDs with delete permissions to default accounts with error if
    /// empty.
    #[deprecated(note = "Use get_default_write_ids_or_error instead")]
    pub fn get_default_delete_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, Some(true))
    }

    /// Filter RelatedAccounts with delete permissions to default accounts with
    /// error if empty.
    #[deprecated(
        note = "Use get_related_account_with_default_write_or_error instead"
    )]
    pub fn get_related_account_with_default_delete_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<RelatedAccounts, MappedErrors> {
        self.get_licensed_ids_as_related_accounts_or_error(
            Permission::Write,
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
        permission: Permission,
        roles: Vec<T>,
        should_be_default: Option<bool>,
    ) -> Vec<Uuid> {
        let inner_licensed_resources =
            if let Some(resources) = &self.licensed_resources {
                match resources {
                    LicensedResources::Records(records) => records,
                    LicensedResources::Urls(urls) => &urls
                        .iter()
                        .map(|i| LicensedResource::from_str(i).unwrap())
                        .collect::<Vec<LicensedResource>>(),
                }
            } else {
                return vec![self.acc_id];
            };

        let licensed_resources = if let Some(true) = should_be_default {
            inner_licensed_resources
                .to_owned()
                .into_iter()
                .filter_map(|license| match license.is_acc_std {
                    true => Some(license.to_owned()),
                    false => None,
                })
                .collect::<Vec<LicensedResource>>()
        } else {
            inner_licensed_resources.to_vec()
        };

        licensed_resources
            .into_iter()
            .filter_map(|i| {
                match i.perm == permission
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
        permission: Permission,
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
            .with_code(NativeErrorCodes::MYC00019)
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
        permission: Permission,
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
            .with_code(NativeErrorCodes::MYC00019)
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
    use super::{LicensedResource, LicensedResources, Owner, Profile};
    use crate::domain::dtos::guest_role::Permission;
    use std::str::FromStr;
    use test_log::test;
    use uuid::Uuid;

    #[test]
    fn profile_get_ids_works() {
        let profile = Profile {
            owners: vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "username@domain.com".to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
                is_principal: true,
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
            licensed_resources: Some(LicensedResources::Records(vec![
                LicensedResource {
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
                    perm: Permission::Write,
                },
            ])),
        };

        let ids = profile.get_write_ids(["service".to_string()].to_vec());

        assert!(ids.len() == 1);
    }

    #[test]
    fn get_licensed_ids_or_error_works() {
        let desired_role = "service".to_string();

        let mut profile = Profile {
            owners: vec![Owner {
                id: Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0")
                    .unwrap(),
                email: "username@domain.com".to_string(),
                first_name: Some("first_name".to_string()),
                last_name: Some("last_name".to_string()),
                username: Some("username".to_string()),
                is_principal: true,
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
            licensed_resources: Some(LicensedResources::Records(vec![
                LicensedResource {
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
                    perm: Permission::Write,
                },
            ])),
        };

        assert_eq!(
            false,
            profile
                .get_read_ids_or_error([desired_role.to_owned()].to_vec())
                .is_ok(),
        );

        assert_eq!(
            false,
            profile
                .get_read_write_ids_or_error([desired_role.to_owned()].to_vec())
                .is_ok(),
        );

        profile.is_manager = true;

        assert_eq!(
            true,
            profile
                .get_write_ids_or_error([desired_role.to_owned()].to_vec())
                .is_ok(),
        );

        profile.is_manager = false;
        profile.is_staff = true;

        assert_eq!(
            true,
            profile
                .get_write_ids_or_error([desired_role.to_owned()].to_vec())
                .is_ok(),
        );

        profile.is_staff = false;

        assert_eq!(
            true,
            profile
                .get_write_ids_or_error([desired_role].to_vec())
                .is_ok(),
        );
    }

    #[test]
    fn test_licensed_resources_from_and_to_string() {
        let licensed_resource = LicensedResource {
            acc_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            acc_name: "Guest Account Name".to_string(),
            is_acc_std: false,
            guest_role_id: Uuid::new_v4(),
            guest_role_name: "guest_role_name".to_string(),
            role: "service".to_string(),
            perm: Permission::Write,
        };

        let licensed_resource_string = licensed_resource.to_string();

        let licensed_resource_parsed =
            LicensedResource::from_str(&licensed_resource_string).unwrap();

        assert_eq!(licensed_resource, licensed_resource_parsed);
    }

    #[test]
    fn test_load_uuid() {
        let uuid_hex_string = "d776e96f94174520b2a99298136031b0";

        let uuid = LicensedResource::load_uuid(uuid_hex_string.to_string());

        assert!(uuid.is_ok());

        assert_eq!(
            Uuid::from_str("d776e96f-9417-4520-b2a9-9298136031b0").unwrap(),
            uuid.unwrap()
        );
    }
}
