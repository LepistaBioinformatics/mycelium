use super::{
    account::VerboseStatus, guest_role::Permission,
    native_error_codes::NativeErrorCodes, related_accounts::RelatedAccounts,
    user::User,
};
use crate::domain::dtos::email::Email;

use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::{dto_err, execution_err, MappedErrors};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct LicensedResource {
    /// The guest account unique id
    ///
    /// This is the unique identifier of the account that is own of the
    /// resource to be managed.
    #[serde(alias = "guest_account_id")]
    pub acc_id: Uuid,

    /// If the guest account is a system account
    ///
    /// System accounts has permissions to act as special users into the
    /// Mycelium system.
    #[serde(alias = "guest_account_is_default")]
    pub sys_acc: bool,

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

    /// If the guest account was verified
    ///
    /// If the user accepted the invitation to join the account, the account
    /// should be verified.
    ///
    pub verified: bool,
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
            "tid/{tenant_id}/aid/{acc_id}?pr={role}:{perm}&sys={is_acc_std}&v={verified}&name={acc_name}",
            tenant_id = self.tenant_id.to_string().replace("-", ""),
            acc_id = self.acc_id.to_string().replace("-", ""),
            role = self.role,
            perm = self.perm.to_owned().to_i32(),
            is_acc_std = self.sys_acc as i8,
            verified = self.verified as i8,
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

        if segments.len() != 4 || segments[0] != "tid" || segments[2] != "aid" {
            return Err("Invalid path format".to_string());
        }

        let tenant_id = segments[1];
        let account_id = segments[3];

        if !Self::is_uuid(tenant_id) {
            return Err("Invalid tenant UUID".to_string());
        }

        if !Self::is_uuid(account_id) {
            return Err("Invalid account UUID".to_string());
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

        let sys = match url
            .query_pairs()
            .find(|(key, _)| key == "sys")
            .map(|(_, value)| value)
            .ok_or("Parameter sys not found")?
            .parse::<i8>()
        {
            Ok(sys) => match sys {
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

        let verified = match url
            .query_pairs()
            .find(|(key, _)| key == "v")
            .map(|(_, value)| value)
            .ok_or("Parameter v not found")?
            .parse::<i8>()
        {
            Ok(verified) => match verified {
                0 => false,
                1 => true,
                _ => {
                    return Err("Invalid account verification".to_string());
                }
            },
            Err(_) => {
                return Err("Failed to parse account verification".to_string());
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
            sys_acc: sys,
            acc_name: String::from_utf8(name_decoded).unwrap(),
            verified,
        })
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum LicensedResources {
    Records(Vec<LicensedResource>),
    Urls(Vec<String>),
}

impl LicensedResources {
    pub fn to_licenses_vector(&self) -> Vec<LicensedResource> {
        match self {
            Self::Records(records) => records.to_owned(),
            Self::Urls(urls) => urls
                .iter()
                .map(|i| LicensedResource::from_str(i).unwrap())
                .collect(),
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq, ToResponse,
)]
pub enum TenantAdmRole {
    Owner,
    Manager,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq, ToResponse,
)]
pub struct TenantAdmDetails {
    /// The tenant ID that the profile has administration privileges
    pub tenant: Uuid,

    /// The tenant administration role
    pub role: TenantAdmRole,

    /// The date and time the tenant was granted to the profile
    pub since: DateTime<Local>,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq, ToResponse,
)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// The owner last name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// The owner username
    #[serde(skip_serializing_if = "Option::is_none")]
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
            email: user.email.email(),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub licensed_resources: Option<LicensedResources>,

    /// Tenants which the profile has ownership
    ///
    /// This field should be used to store the tenants that the profile has
    /// ownership. The ownership should be used to filter the licensed resources
    /// during system validations.
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenants_with_adm: Option<Vec<TenantAdmDetails>>,

    /// This argument stores the licensed resources state
    ///
    /// The licensed_resources_state should store the current filtering state.
    /// The filtering state should be populated when a filtering cascade is
    /// performed. As example:
    ///
    /// If a profile with two licensed resources is filtered by the tenant_id
    /// the state should store the tenant id used to filter licensed resources.
    ///
    /// State formatting:
    ///
    /// ```json
    /// [
    ///    "1:tenantId:123e4567-e89b-12d3-a456-426614174000",
    /// ]
    /// ```
    ///
    /// And then, if the used apply a secondary filter, by permission, the state
    /// should be updated to:
    ///
    /// ```json
    /// [
    ///   "1:tenantId:123e4567-e89b-12d3-a456-426614174000",
    ///   "2:permission:1",
    /// ]
    /// ```
    ///
    /// If a consecutive filter with more one tenant is applied, the state
    /// should be updated to:
    ///
    /// ```json
    /// [
    ///  "1:tenantId:123e4567-e89b-12d3-a456-426614174000",
    ///  "2:permission:1",
    ///  "3:tenantId:123e4567-e89b-12d3-a456-426614174001",
    /// ]
    /// ```
    ///
    #[serde(skip_serializing_if = "Option::is_none")]
    licensed_resources_state: Option<Vec<String>>,
}

impl Profile {
    pub fn new(
        owners: Vec<Owner>,
        acc_id: Uuid,
        is_subscription: bool,
        is_manager: bool,
        is_staff: bool,
        owner_is_active: bool,
        account_is_active: bool,
        account_was_approved: bool,
        account_was_archived: bool,
        verbose_status: Option<VerboseStatus>,
        licensed_resources: Option<LicensedResources>,
        tenants_with_adm: Option<Vec<TenantAdmDetails>>,
    ) -> Self {
        Self {
            owners,
            acc_id,
            is_subscription,
            is_manager,
            is_staff,
            owner_is_active,
            account_is_active,
            account_was_approved,
            account_was_archived,
            verbose_status,
            licensed_resources,
            tenants_with_adm,
            licensed_resources_state: None,
        }
    }

    fn update_state(&self, key: String, value: String) -> Self {
        let mut state =
            self.licensed_resources_state.clone().unwrap_or_default();

        state.push(format!(
            "{}:{}",
            state.len() + 1,
            format!("{}:{}", key, value)
        ));

        Self {
            licensed_resources_state: Some(state),
            ..self.clone()
        }
    }

    pub fn profile_string(&self) -> String {
        format!("profile/{}", self.acc_id.to_string())
    }

    /// Redacted profile string
    ///
    /// Print the profile using the profile_string struct method and a list of
    /// owners, using the `redacted_email` structural method of the email field
    /// present in owners.
    ///
    pub fn profile_redacted(&self) -> String {
        format!(
            "profile/{} owners: [{}]",
            self.acc_id.to_string(),
            self.owners
                .iter()
                .map(|i| Email::from_string(i.email.to_owned())
                    .unwrap()
                    .redacted_email())
                .collect::<Vec<String>>()
                .join(", ")
        )
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
                let records: Vec<LicensedResource> = resources
                    .to_licenses_vector()
                    .iter()
                    .filter(|i| i.tenant_id == tenant_id)
                    .map(|i| i.to_owned())
                    .collect();

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
            ..self
                .update_state("tenantId".to_string(), tenant_id.to_string())
                .clone()
        }
    }

    /// Filter the licensed resources to include only the standard system
    /// accounts
    pub fn with_system_accounts_access(&self) -> Self {
        //
        // Filter the licensed resources to the default accounts
        //
        let licensed_resources =
            if let Some(resources) = self.licensed_resources.as_ref() {
                let records: Vec<LicensedResource> = resources
                    .to_licenses_vector()
                    .iter()
                    .filter(|i| i.sys_acc == true)
                    .map(|i| i.to_owned())
                    .collect();

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
            ..self
                .update_state("isAccStd".to_string(), "true".to_string())
                .clone()
        }
    }

    /// Filter the licensed resources to include only licenses with read access
    pub fn with_read_access(&self) -> Self {
        self.with_permission(Permission::Read)
    }

    /// Filter the licensed resources to include only licenses with write access
    pub fn with_write_access(&self) -> Self {
        self.with_permission(Permission::Write)
    }

    /// Filter the licensed resources to include only licenses with read/write
    pub fn with_read_write_access(&self) -> Self {
        self.with_permission(Permission::ReadWrite)
    }

    /// Filter licensed resources by permission
    ///
    /// This is an internal method that should be used to filter the licensed
    /// resources by permission.
    ///
    fn with_permission(&self, permission: Permission) -> Self {
        //
        // Filter the licensed resources to the permission
        //
        let licensed_resources =
            if let Some(resources) = self.licensed_resources.as_ref() {
                let records: Vec<LicensedResource> = resources
                    .to_licenses_vector()
                    .iter()
                    .filter(|i| i.perm == permission)
                    .map(|i| i.to_owned())
                    .collect();

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
            ..self
                .update_state(
                    "permission".to_string(),
                    permission.to_i32().to_string(),
                )
                .clone()
        }
    }

    pub fn with_roles<T: ToString>(&self, roles: Vec<T>) -> Self {
        //
        // Filter the licensed resources to the roles
        //
        let licensed_resources =
            if let Some(resources) = self.licensed_resources.as_ref() {
                let records: Vec<LicensedResource> = resources
                    .to_licenses_vector()
                    .iter()
                    .filter(|i| {
                        roles
                            .iter()
                            .map(|i| i.to_string())
                            .collect::<Vec<String>>()
                            .contains(&i.role)
                    })
                    .map(|i| i.to_owned())
                    .collect();

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
            ..self
                .update_state(
                    "role".to_string(),
                    roles
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<String>>()
                        .join(","),
                )
                .clone()
        }
    }

    pub fn get_related_account_or_error(
        &self,
    ) -> Result<RelatedAccounts, MappedErrors> {
        if self.is_staff {
            return Ok(RelatedAccounts::HasStaffPrivileges);
        }

        if self.is_manager {
            return Ok(RelatedAccounts::HasManagerPrivileges);
        }

        if let Some(resources) = self.licensed_resources.as_ref() {
            let records: Vec<LicensedResource> = resources.to_licenses_vector();

            if records.is_empty() {
                return execution_err(
                    "Insufficient licenses to perform these action".to_string(),
                )
                .with_code(NativeErrorCodes::MYC00019)
                .with_exp_true()
                .as_error();
            }

            return Ok(RelatedAccounts::AllowedAccounts(
                records.iter().map(|i| i.acc_id).collect(),
            ));
        }

        execution_err(format!(
            "Insufficient privileges to perform these action (no accounts): {}",
            self.licensed_resources_state
                .to_owned()
                .unwrap_or(vec![])
                .join(", ")
        ))
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error()
    }

    pub fn get_ids_or_error(&self) -> Result<Vec<Uuid>, MappedErrors> {
        let ids: Vec<Uuid> = self
            .licensed_resources
            .to_owned()
            .unwrap_or(LicensedResources::Records(vec![]))
            .to_licenses_vector()
            .iter()
            .map(|i| i.acc_id)
            .collect();

        //
        // If none of the conditions are true, return an error
        //
        if !vec![
            //
            // The profile has more than one licensed resource and the profile
            // is not the owner of the account
            //
            ids.len() > 0,
            //
            // The profile has no staff privileges
            //
            self.is_staff,
            //
            // The profile has no manager privileges
            //
            self.is_manager,
        ]
        .into_iter()
        .any(|i| i == true)
        {
            return execution_err(format!(
                "Insufficient privileges to perform these action (no ids): {}",
                self.licensed_resources_state
                    .to_owned()
                    .unwrap_or(vec![])
                    .join(", ")
            ))
            .with_code(NativeErrorCodes::MYC00019)
            .with_exp_true()
            .as_error();
        }

        Ok(ids)
    }

    // ? -----------------------------------------------------------------------
    // ? Read filters
    // ? -----------------------------------------------------------------------

    /// Filter IDs with read permissions.
    #[deprecated(note = "To be removed in the future")]
    pub fn get_read_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::Read, roles, None)
    }

    /// Filter IDs with read permissions with error if empty.
    #[deprecated(note = "To be removed in the future")]
    pub fn get_read_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Read, roles, None)
    }

    /// Filter IDs with read permissions to accounts with error if empty.
    #[deprecated(note = "To be removed in the future")]
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
    #[deprecated(note = "To be removed in the future")]
    pub fn get_default_read_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Read, roles, Some(true))
    }

    /// Filter RelatedAccounts with read permissions to default accounts with
    /// error if empty.
    #[deprecated(note = "To be removed in the future")]
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
    #[deprecated(note = "To be removed in the future")]
    pub fn get_write_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::Write, roles, None)
    }

    /// Filter IDs with write permissions with error if empty.
    #[deprecated(note = "To be removed in the future")]
    pub fn get_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, None)
    }

    /// Filter IDs with write permissions to accounts with error if empty.
    #[deprecated(note = "To be removed in the future")]
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
    #[deprecated(note = "To be removed in the future")]
    pub fn get_default_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::Write, roles, Some(true))
    }

    /// Filter RelatedAccounts with write permissions to default accounts with
    /// error if empty.
    #[deprecated(note = "To be removed in the future")]
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
    #[deprecated(note = "To be removed in the future")]
    pub fn get_read_write_ids<T: ToString>(&self, roles: Vec<T>) -> Vec<Uuid> {
        self.get_licensed_ids(Permission::ReadWrite, roles, None)
    }

    /// Filter IDs with write permissions with error if empty.
    #[deprecated(note = "To be removed in the future")]
    pub fn get_read_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::ReadWrite, roles, None)
    }

    /// Filter IDs with write permissions to accounts with error if empty.
    #[deprecated(note = "To be removed in the future")]
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
    #[deprecated(note = "To be removed in the future")]
    pub fn get_default_read_write_ids_or_error<T: ToString>(
        &self,
        roles: Vec<T>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        self.get_licensed_ids_or_error(Permission::ReadWrite, roles, Some(true))
    }

    /// Filter RelatedAccounts with write permissions to default accounts with
    /// error if empty.
    #[deprecated(note = "To be removed in the future")]
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
    #[deprecated(note = "To be removed in the future")]
    fn get_licensed_ids<T: ToString>(
        &self,
        permission: Permission,
        roles: Vec<T>,
        should_be_default: Option<bool>,
    ) -> Vec<Uuid> {
        let inner_licensed_resources =
            if let Some(resources) = &self.licensed_resources {
                resources.to_licenses_vector()
            } else {
                //
                // WARNING: If the licensed resources are empty, the profile
                // should be the owner of the account.
                //
                return vec![self.acc_id];
            };

        let licensed_resources = if let Some(true) = should_be_default {
            inner_licensed_resources
                .to_owned()
                .into_iter()
                .filter_map(|license| match license.sys_acc {
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
                //
                // Check if the desired permission is the same as the license
                //
                match i.perm == permission
                    //
                    // Check if the license was already verified
                    //
                    && i.verified == true
                    //
                    // Check if the license contains the desired role
                    //
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

    #[deprecated(note = "To be removed in the future")]
    fn get_licensed_ids_or_error<T: ToString>(
        &self,
        permission: Permission,
        roles: Vec<T>,
        should_be_default: Option<bool>,
    ) -> Result<Vec<Uuid>, MappedErrors> {
        let ids = self.get_licensed_ids(permission, roles, should_be_default);

        //
        // If none of the conditions are true, return an error
        //
        if !vec![
            //
            // The profile has more than one licensed resource and the profile
            // is not the owner of the account
            //
            ids.len() > 0,
            //
            // The profile has no staff privileges
            //
            self.is_staff,
            //
            // The profile has no manager privileges
            //
            self.is_manager,
        ]
        .into_iter()
        .any(|i| i == true)
        {
            return execution_err(
                format!(
                "Insufficient privileges to perform these action (no licenses): {}",
                self.licensed_resources_state
                    .to_owned()
                    .unwrap_or(vec![])
                    .join(", ")
            ),
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
    #[deprecated(note = "To be removed in the future")]
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
                format!(
                "Insufficient privileges to perform these action (no guesting): {}",
                self.licensed_resources_state
                    .to_owned()
                    .unwrap_or(vec![])
                    .join(", ")
                ),
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

    // Define the tenant_id to share between tests
    fn tenant_id() -> Uuid {
        Uuid::from_str("e497848f-a0d4-49f4-8288-c3df11416ff1").unwrap()
    }

    fn profile() -> Profile {
        let tenant_id = tenant_id();

        Profile {
            owners: vec![],
            acc_id: Uuid::new_v4(),
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
                    acc_id: Uuid::new_v4(),
                    tenant_id,
                    acc_name: "Guest Account Name".to_string(),
                    sys_acc: false,
                    role: "service".to_string(),
                    perm: Permission::Write,
                    verified: true,
                },
                LicensedResource {
                    acc_id: Uuid::new_v4(),
                    tenant_id,
                    acc_name: "Guest Account Name".to_string(),
                    sys_acc: true,
                    role: "newbie".to_string(),
                    perm: Permission::Read,
                    verified: true,
                },
                LicensedResource {
                    acc_id: Uuid::new_v4(),
                    tenant_id: Uuid::new_v4(),
                    acc_name: "Guest Account Name".to_string(),
                    sys_acc: true,
                    role: "service".to_string(),
                    perm: Permission::ReadWrite,
                    verified: true,
                },
            ])),
            tenants_with_adm: None,
            licensed_resources_state: None,
        }
    }

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
                    sys_acc: false,
                    role: "service".to_string(),
                    perm: Permission::Write,
                    verified: true,
                },
            ])),
            tenants_with_adm: None,
            licensed_resources_state: None,
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
                    sys_acc: false,
                    role: "service".to_string(),
                    perm: Permission::Write,
                    verified: true,
                },
            ])),
            tenants_with_adm: None,
            licensed_resources_state: None,
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
            sys_acc: false,
            role: "service".to_string(),
            perm: Permission::Write,
            verified: true,
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

    #[test]
    fn test_filtering_permissions() {
        let profile = profile();
        let profile_with_read = profile.with_read_access();
        let profile_with_write = profile.with_write_access();
        let profile_with_read_write = profile.with_read_write_access();
        let profile_with_standard = profile.with_system_accounts_access();

        assert_eq!(
            1,
            profile_with_read
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        assert_eq!(
            1,
            profile_with_write
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        assert_eq!(
            1,
            profile_with_read_write
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        assert_eq!(
            2,
            profile_with_standard
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );
    }

    #[test]
    fn test_filtering_on_tenant_cascade() {
        let tenant_id = tenant_id();
        let profile = profile();

        let profile_on_tenant = profile.on_tenant(tenant_id);

        assert_eq!(
            2,
            profile_on_tenant
                .licensed_resources
                .clone()
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        let profile_on_tenant_with_read = profile_on_tenant.with_read_access();
        let profile_on_tenant_with_write =
            profile_on_tenant.with_write_access();
        let profile_on_tenant_with_read_write =
            profile_on_tenant.with_read_write_access();
        assert_eq!(
            1,
            profile_on_tenant_with_read
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        assert_eq!(
            1,
            profile_on_tenant_with_write
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        assert!(profile_on_tenant_with_read_write
            .licensed_resources
            .is_none());
    }

    #[test]
    fn test_filtering_by_role() {
        let tenant_id = tenant_id();
        let profile = profile();

        let profile_on_tenant = profile.on_tenant(tenant_id);

        let profile_on_tenant_with_roles =
            profile_on_tenant.with_roles(["service".to_string()].to_vec());

        assert_eq!(
            1,
            profile_on_tenant_with_roles
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        let profile_on_tenant_with_roles =
            profile_on_tenant.with_roles(["newbie".to_string()].to_vec());

        assert_eq!(
            1,
            profile_on_tenant_with_roles
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );

        let profile_on_tenant_with_roles =
            profile_on_tenant.with_roles(["service", "newbie"].to_vec());

        assert_eq!(
            2,
            profile_on_tenant_with_roles
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );
    }

    #[test]
    fn test_filtering_as_default() {
        let tenant_id = tenant_id();
        let profile = profile();
        let profile_on_tenant = profile.on_tenant(tenant_id);

        let profile_on_tenant_with_standard =
            profile_on_tenant.with_system_accounts_access();

        assert_eq!(
            1,
            profile_on_tenant_with_standard
                .licensed_resources
                .unwrap()
                .to_licenses_vector()
                .len()
        );
    }
}
