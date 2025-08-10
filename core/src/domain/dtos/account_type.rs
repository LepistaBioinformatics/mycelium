use crate::domain::actors::SystemActor;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountType {
    /// Staff account type
    ///
    /// Staff account type is a special account type that is used to represent
    /// staff members in the system.
    Staff,

    /// Manager account type
    ///
    /// Manager account type is a special account type that is used to represent
    /// managers in the system.
    Manager,

    /// User account type
    ///
    /// User account type is the default account type for users in the system.
    User,

    /// Subscription account type
    ///
    /// A subscription account is a special account type that is used to
    /// represent legal entities that have a subscription to the service.
    #[serde(rename_all = "camelCase")]
    Subscription { tenant_id: Uuid },

    /// Role associated account type
    ///
    /// Role associated account type is an special type of account, created to
    /// connect users to a specific standard role in the application.
    #[serde(rename_all = "camelCase")]
    RoleAssociated {
        /// The tenant ID
        tenant_id: Uuid,

        /// The role name
        ///
        /// The role name should be the same for the read and write roles.
        ///
        role_name: String,

        /// The read role ID
        ///
        /// The read role ID is the ID of the role that will be used to read the
        /// data from the account.
        ///
        read_role_id: Uuid,

        /// The write role ID
        ///
        /// The write role ID is the ID of the role that will be used to write
        /// the data to the account.
        ///
        write_role_id: Uuid,
    },

    /// Actor associated account type
    #[serde(rename_all = "camelCase")]
    ActorAssociated { actor: SystemActor },

    /// Tenant manager account type
    #[serde(rename_all = "camelCase")]
    TenantManager { tenant_id: Uuid },
}

impl AccountType {
    pub fn is_tenant_dependent(&self) -> bool {
        matches!(
            self,
            AccountType::Subscription { .. }
                | AccountType::RoleAssociated { .. }
                | AccountType::TenantManager { .. }
        )
    }

    pub fn is_user_account(&self) -> bool {
        matches!(
            self,
            AccountType::User | AccountType::Staff | AccountType::Manager
        )
    }

    pub fn is_system_default_account(&self) -> bool {
        matches!(
            self,
            AccountType::ActorAssociated { .. }
                | AccountType::RoleAssociated { .. }
        )
    }
}

impl ToString for AccountType {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_type_v2_json_serialization() {
        //
        // Type AccountTypeV2::Subscription
        //
        let account_type = AccountType::Subscription {
            tenant_id: Uuid::from_u128(0),
        };

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(
            json,
            r#"{"subscription":{"tenantId":"00000000-0000-0000-0000-000000000000"}}"#
        );

        let account_type: AccountType = serde_json::from_str(&json).unwrap();

        assert_eq!(
            account_type,
            AccountType::Subscription {
                tenant_id: Uuid::from_u128(0)
            }
        );

        //
        // Type AccountTypeV2::StandardRoleAssociated
        //
        let account_type = AccountType::RoleAssociated {
            tenant_id: Uuid::from_u128(0),
            role_name: SystemActor::CustomRole("test".to_string()).to_string(),
            read_role_id: Uuid::from_u128(0),
            write_role_id: Uuid::from_u128(0),
        };

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(
            json,
            r#"{"roleAssociated":{"tenantId":"00000000-0000-0000-0000-000000000000","roleName":"custom-role:test","readRoleId":"00000000-0000-0000-0000-000000000000","writeRoleId":"00000000-0000-0000-0000-000000000000"}}"#
        );

        let account_type: AccountType = serde_json::from_str(&json).unwrap();

        assert_eq!(
            account_type,
            AccountType::RoleAssociated {
                tenant_id: Uuid::from_u128(0),
                role_name: SystemActor::CustomRole("test".to_string())
                    .to_string(),
                read_role_id: Uuid::from_u128(0),
                write_role_id: Uuid::from_u128(0),
            }
        );

        //
        // Type AccountTypeV2::TenantManager
        //
        let account_type = AccountType::TenantManager {
            tenant_id: Uuid::from_u128(0),
        };

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(
            json,
            r#"{"tenantManager":{"tenantId":"00000000-0000-0000-0000-000000000000"}}"#
        );

        let account_type: AccountType = serde_json::from_str(&json).unwrap();

        assert_eq!(
            account_type,
            AccountType::TenantManager {
                tenant_id: Uuid::from_u128(0)
            }
        );

        //
        // Type AccountTypeV2::Manager
        //
        let account_type = AccountType::Manager;

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(json, r#""manager""#);

        let account_type: AccountType = serde_json::from_str(&json).unwrap();

        assert_eq!(account_type, AccountType::Manager);

        //
        // Type AccountTypeV2::Staff
        //
        let account_type = AccountType::Staff;

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(json, r#""staff""#);

        let account_type: AccountType = serde_json::from_str(&json).unwrap();

        assert_eq!(account_type, AccountType::Staff);

        //
        // Type AccountTypeV2::User
        //
        let account_type = AccountType::User;

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(json, r#""user""#);
    }
}
