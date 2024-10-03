use crate::domain::actors::ActorName;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountTypeV2 {
    Manager,
    Staff,

    /// User account type
    ///
    /// User account type is the default account type for users in the system.
    User,

    /// Subscription account type
    ///
    /// A subscription account is a special account type that is used to
    /// represent legal entities that have a subscription to the service.
    Subscription {
        tenant_id: Uuid,
    },

    /// Role associated account type
    ///
    /// Role associated account type is an special type of account, created to
    /// connect users to a specific standard role in the application.
    StandardRoleAssociated {
        tenant_id: Uuid,
        role_name: ActorName,
        role_id: Uuid,
    },

    TenantManager {
        tenant_id: Uuid,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_type_v2_json_serialization() {
        let account_type = AccountTypeV2::Subscription {
            tenant_id: Uuid::from_u128(0),
        };

        let json = serde_json::to_string(&account_type).unwrap();

        assert_eq!(
            json,
            r#"{"subscription":{"tenantId":"00000000-0000-0000-0000-000000000000"}}"#
        );
    }
}
