mod licensed_resources;
mod owner;
mod tenant_ownerships;

pub use licensed_resources::{LicensedResource, LicensedResources};
pub use owner::Owner;
pub use tenant_ownerships::{TenantOwnership, TenantsOwnership};

use super::{
    account::VerboseStatus, guest_role::Permission,
    native_error_codes::NativeErrorCodes, related_accounts::RelatedAccounts,
};
use crate::domain::dtos::email::Email;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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

    /// If the account was deleted after registration
    ///
    /// New accounts should be deleted. After deleted accounts should not be
    /// included at default filtering actions.
    pub account_was_deleted: bool,

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
    pub tenants_ownership: Option<TenantsOwnership>,

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
    filtering_state: Option<Vec<String>>,
}

impl Default for Profile {
    fn default() -> Self {
        Self::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }
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
        account_was_deleted: bool,
        verbose_status: Option<VerboseStatus>,
        licensed_resources: Option<LicensedResources>,
        tenants_ownership: Option<TenantsOwnership>,
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
            account_was_deleted,
            verbose_status,
            licensed_resources,
            tenants_ownership,
            filtering_state: None,
        }
    }

    fn update_state(&self, key: String, value: String) -> Self {
        let mut state = self.filtering_state.clone().unwrap_or_default();

        state.push(format!(
            "{}:{}",
            state.len() + 1,
            format!("{}:{}", key, value)
        ));

        Self {
            filtering_state: Some(state),
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

    /// Filter the licensed resources to the account
    ///
    /// This method should be used to filter licensed resources to the account
    /// that the profile is currently working on.
    pub fn on_account(&self, account_id: Uuid) -> Self {
        let licensed_resources =
            if let Some(resources) = self.licensed_resources.as_ref() {
                let records: Vec<LicensedResource> = resources
                    .to_licenses_vector()
                    .iter()
                    .filter(|i| i.acc_id == account_id)
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

        Self {
            licensed_resources,
            ..self
                .update_state("accountId".to_string(), account_id.to_string())
                .clone()
        }
    }

    /// Filter the tenant ownership by the tenant
    pub fn with_tenant_ownership_or_error(
        &self,
        tenant_id: Uuid,
    ) -> Result<Self, MappedErrors> {
        if let Some(tenants) = self.tenants_ownership.as_ref() {
            let tenants = tenants.to_ownership_vector();

            if tenants.iter().any(|i| i.tenant == tenant_id) {
                return Ok(self.to_owned());
            }
        }

        execution_err(format!(
            "Insufficient privileges to perform these action (no tenant ownership): {}",
            self.filtering_state.to_owned().unwrap_or(vec![]).join(", ")
        ))
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error()
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
                    .filter(|i| i.perm.to_i32() >= permission.to_i32())
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
            self.filtering_state.to_owned().unwrap_or(vec![]).join(", ")
        ))
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error()
    }

    pub fn get_related_accounts_or_tenant_or_error(
        &self,
        tenant_id: Uuid,
    ) -> Result<RelatedAccounts, MappedErrors> {
        if self.is_staff {
            return Ok(RelatedAccounts::HasStaffPrivileges);
        }

        if self.is_manager {
            return Ok(RelatedAccounts::HasManagerPrivileges);
        }

        if let Some(tenants) = self.tenants_ownership.as_ref() {
            let tenants = tenants.to_ownership_vector();

            if tenants.iter().any(|i| i.tenant == tenant_id) {
                return Ok(RelatedAccounts::HasTenantWidePrivileges(tenant_id));
            }
        }

        self.get_related_account_or_error()
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
                self.filtering_state.to_owned().unwrap_or(vec![]).join(", ")
            ))
            .with_code(NativeErrorCodes::MYC00019)
            .with_exp_true()
            .as_error();
        }

        Ok(ids)
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{
        LicensedResource, LicensedResources, Profile, TenantOwnership,
        TenantsOwnership,
    };
    use crate::domain::dtos::{
        guest_role::Permission, related_accounts::RelatedAccounts,
    };
    use chrono::Local;
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
            account_was_deleted: false,
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
                    perm: Permission::Write,
                    verified: true,
                },
            ])),
            tenants_ownership: Some(TenantsOwnership::Records(vec![
                TenantOwnership {
                    tenant: tenant_id,
                    since: Local::now(),
                },
            ])),
            filtering_state: None,
        }
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

    #[test]
    fn test_with_tenant_ownership_or_error() {
        let tenant_id = tenant_id();
        let profile = profile();

        let profile_on_tenant = profile.on_tenant(tenant_id);

        assert!(profile_on_tenant
            .with_tenant_ownership_or_error(tenant_id)
            .is_ok());

        assert!(profile_on_tenant
            .with_tenant_ownership_or_error(Uuid::new_v4())
            .is_err());
    }

    #[test]
    fn test_get_my_account_details() {
        let profile = profile();
        let result_ok = profile.get_related_account_or_error();

        assert!(result_ok.is_ok());

        let related_accounts = result_ok.unwrap();

        assert_ne!(related_accounts, RelatedAccounts::HasManagerPrivileges);
        assert_ne!(related_accounts, RelatedAccounts::HasStaffPrivileges);

        let account_ids = match related_accounts {
            RelatedAccounts::AllowedAccounts(ids) => ids,
            _ => vec![],
        };

        let licensed_resources = profile.licensed_resources.unwrap();

        let licensed_resources_ids = licensed_resources.to_licenses_vector();

        assert_eq!(account_ids.len(), licensed_resources_ids.len());
        assert!(!account_ids.contains(&profile.acc_id));
    }
}
