// ? ---------------------------------------------------------------------------
// ? ResourceAuditResourceType
//
// Coarse resource categories tracked by the resource audit log. Granularity
// intentionally stops at "which kind of resource", not "which use case fired"
// -- finer detail (e.g. "subscription account created") belongs in
// `ResourceAuditLog::metadata`, not in an ever-growing enum.
// ? ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ResourceAuditResourceType {
    /// An `account` row was affected.
    Account,

    /// Account metadata (e.g. status, tags) was affected.
    AccountMeta,

    /// A `user` row was affected.
    User,

    /// A `tenant` row was affected.
    Tenant,

    /// Tenant metadata was affected.
    TenantMeta,

    /// A `guest_role` row was affected.
    GuestRole,

    /// A `webhook` row was affected.
    Webhook,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_audit_resource_type_round_trips_through_json() {
        let variants = [
            ResourceAuditResourceType::Account,
            ResourceAuditResourceType::AccountMeta,
            ResourceAuditResourceType::User,
            ResourceAuditResourceType::Tenant,
            ResourceAuditResourceType::TenantMeta,
            ResourceAuditResourceType::GuestRole,
            ResourceAuditResourceType::Webhook,
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let parsed: ResourceAuditResourceType =
                serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn resource_audit_resource_type_uses_camel_case() {
        let json =
            serde_json::to_string(&ResourceAuditResourceType::AccountMeta)
                .unwrap();
        assert_eq!(json, r#""accountMeta""#);
    }
}
