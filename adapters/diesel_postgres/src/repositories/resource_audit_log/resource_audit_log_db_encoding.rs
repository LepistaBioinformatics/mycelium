// ? ---------------------------------------------------------------------------
// ? resource_audit_log DB encoding
//
// `ResourceAuditResourceType`/`ResourceAuditEventKind` serialize as camelCase
// via serde (API-facing convention), but the `resource_audit_log` table's
// `CHECK` constraints expect snake_case (`account_meta`, not `accountMeta`).
// These free functions are the explicit mapping between the two -- kept here,
// adapter-side, instead of on the core enums, since the orphan rule forbids
// implementing `std::str::FromStr`/`Display` for a foreign type from this
// crate anyway, and the snake_case shape is a DB-only concern.
// ? ---------------------------------------------------------------------------

use myc_core::domain::dtos::resource_audit_log::{
    ResourceAuditEventKind, ResourceAuditResourceType,
};

// Consumed by `append_resource_audit_log_row` when it builds a
// `NewResourceAuditLog` row to insert.
pub(super) fn resource_type_to_db_str(
    resource_type: &ResourceAuditResourceType,
) -> &'static str {
    match resource_type {
        ResourceAuditResourceType::Account => "account",
        ResourceAuditResourceType::AccountMeta => "account_meta",
        ResourceAuditResourceType::User => "user",
        ResourceAuditResourceType::Tenant => "tenant",
        ResourceAuditResourceType::TenantMeta => "tenant_meta",
        ResourceAuditResourceType::GuestRole => "guest_role",
        ResourceAuditResourceType::Webhook => "webhook",
    }
}

pub(super) fn resource_type_from_db_str(
    value: &str,
) -> Result<ResourceAuditResourceType, String> {
    match value {
        "account" => Ok(ResourceAuditResourceType::Account),
        "account_meta" => Ok(ResourceAuditResourceType::AccountMeta),
        "user" => Ok(ResourceAuditResourceType::User),
        "tenant" => Ok(ResourceAuditResourceType::Tenant),
        "tenant_meta" => Ok(ResourceAuditResourceType::TenantMeta),
        "guest_role" => Ok(ResourceAuditResourceType::GuestRole),
        "webhook" => Ok(ResourceAuditResourceType::Webhook),
        other => {
            Err(format!("Unknown resource_audit_log resource_type: {other}"))
        }
    }
}

// Consumed by `append_resource_audit_log_row` when it builds a
// `NewResourceAuditLog` row to insert.
pub(super) fn event_kind_to_db_str(
    event: &ResourceAuditEventKind,
) -> &'static str {
    match event {
        ResourceAuditEventKind::Created => "created",
        ResourceAuditEventKind::Updated => "updated",
        ResourceAuditEventKind::Deleted => "deleted",
    }
}

pub(super) fn event_kind_from_db_str(
    value: &str,
) -> Result<ResourceAuditEventKind, String> {
    match value {
        "created" => Ok(ResourceAuditEventKind::Created),
        "updated" => Ok(ResourceAuditEventKind::Updated),
        "deleted" => Ok(ResourceAuditEventKind::Deleted),
        other => Err(format!("Unknown resource_audit_log event: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_type_round_trips_through_db_str() {
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
            let db_str = resource_type_to_db_str(&variant);
            let parsed = resource_type_from_db_str(db_str).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn resource_type_uses_snake_case() {
        assert_eq!(
            resource_type_to_db_str(&ResourceAuditResourceType::AccountMeta),
            "account_meta"
        );
    }

    #[test]
    fn event_kind_round_trips_through_db_str() {
        let variants = [
            ResourceAuditEventKind::Created,
            ResourceAuditEventKind::Updated,
            ResourceAuditEventKind::Deleted,
        ];

        for variant in variants {
            let db_str = event_kind_to_db_str(&variant);
            let parsed = event_kind_from_db_str(db_str).unwrap();
            assert_eq!(parsed, variant);
        }
    }
}
