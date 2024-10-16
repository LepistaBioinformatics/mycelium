use super::tenant::TenantId;
use crate::domain::actors::ActorName;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountTypeV2 {
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
    Subscription { tenant_id: TenantId },

    /// Role associated account type
    ///
    /// Role associated account type is an special type of account, created to
    /// connect users to a specific standard role in the application.
    #[serde(rename_all = "camelCase")]
    StandardRoleAssociated {
        tenant_id: TenantId,
        role_name: ActorName,
        role_id: Uuid,
    },

    #[serde(rename_all = "camelCase")]
    TenantManager { tenant_id: TenantId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_type_v2_json_serialization() {
        //
        // Type AccountTypeV2::Subscription
        //
        let account_type = AccountTypeV2::Subscription {
            tenant_id: TenantId::from_u128(0),
        };

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(
            json,
            r#"{"subscription":{"tenantId":"00000000-0000-0000-0000-000000000000"}}"#
        );

        let account_type: AccountTypeV2 = serde_json::from_str(&json).unwrap();

        assert_eq!(
            account_type,
            AccountTypeV2::Subscription {
                tenant_id: TenantId::from_u128(0)
            }
        );

        //
        // Type AccountTypeV2::StandardRoleAssociated
        //
        let account_type = AccountTypeV2::StandardRoleAssociated {
            tenant_id: TenantId::from_u128(0),
            role_name: ActorName::CustomRole("test".to_string()),
            role_id: Uuid::from_u128(0),
        };

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(
            json,
            r#"{"standardRoleAssociated":{"tenantId":"00000000-0000-0000-0000-000000000000","roleName":{"customRole":"test"},"roleId":"00000000-0000-0000-0000-000000000000"}}"#
        );

        let account_type: AccountTypeV2 = serde_json::from_str(&json).unwrap();

        assert_eq!(
            account_type,
            AccountTypeV2::StandardRoleAssociated {
                tenant_id: TenantId::from_u128(0),
                role_name: ActorName::CustomRole("test".to_string()),
                role_id: Uuid::from_u128(0),
            }
        );

        //
        // Type AccountTypeV2::TenantManager
        //
        let account_type = AccountTypeV2::TenantManager {
            tenant_id: TenantId::from_u128(0),
        };

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(
            json,
            r#"{"tenantManager":{"tenantId":"00000000-0000-0000-0000-000000000000"}}"#
        );

        let account_type: AccountTypeV2 = serde_json::from_str(&json).unwrap();

        assert_eq!(
            account_type,
            AccountTypeV2::TenantManager {
                tenant_id: TenantId::from_u128(0)
            }
        );

        //
        // Type AccountTypeV2::Manager
        //
        let account_type = AccountTypeV2::Manager;

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(json, r#""manager""#);

        let account_type: AccountTypeV2 = serde_json::from_str(&json).unwrap();

        assert_eq!(account_type, AccountTypeV2::Manager);

        //
        // Type AccountTypeV2::Staff
        //
        let account_type = AccountTypeV2::Staff;

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(json, r#""staff""#);

        let account_type: AccountTypeV2 = serde_json::from_str(&json).unwrap();

        assert_eq!(account_type, AccountTypeV2::Staff);

        //
        // Type AccountTypeV2::User
        //
        let account_type = AccountTypeV2::User;

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(json, r#""user""#);
    }
}
