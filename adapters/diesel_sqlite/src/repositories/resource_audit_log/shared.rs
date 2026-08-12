use crate::{
    models::resource_audit_log::ResourceAuditLog as ResourceAuditLogModel,
    types::{json_from_text, timestamp_from_text, uuid_from_text},
};

use myc_core::domain::dtos::{
    resource_audit_log::{
        ResourceAuditEventKind, ResourceAuditLog, ResourceAuditResourceType,
    },
    written_by::WrittenBy,
};
use mycelium_base::utils::errors::{dto_err, MappedErrors};

// ? ----------------------------------------------------------------------------
// ? resource_type / event <-> snake_case TEXT
// ?
// ? The SQL `CHECK` constraint on both columns expects snake_case (see the
// ? migration), while the DTOs serialize as camelCase via serde. These
// ? explicit mappings keep the DB encoding independent of the serde
// ? representation.
// ? ----------------------------------------------------------------------------

pub(crate) fn resource_type_to_text(
    value: &ResourceAuditResourceType,
) -> &'static str {
    match value {
        ResourceAuditResourceType::Account => "account",
        ResourceAuditResourceType::AccountMeta => "account_meta",
        ResourceAuditResourceType::User => "user",
        ResourceAuditResourceType::Tenant => "tenant",
        ResourceAuditResourceType::TenantMeta => "tenant_meta",
        ResourceAuditResourceType::GuestRole => "guest_role",
        ResourceAuditResourceType::Webhook => "webhook",
    }
}

fn resource_type_from_text(
    value: &str,
) -> Result<ResourceAuditResourceType, MappedErrors> {
    match value {
        "account" => Ok(ResourceAuditResourceType::Account),
        "account_meta" => Ok(ResourceAuditResourceType::AccountMeta),
        "user" => Ok(ResourceAuditResourceType::User),
        "tenant" => Ok(ResourceAuditResourceType::Tenant),
        "tenant_meta" => Ok(ResourceAuditResourceType::TenantMeta),
        "guest_role" => Ok(ResourceAuditResourceType::GuestRole),
        "webhook" => Ok(ResourceAuditResourceType::Webhook),
        other => Err(dto_err(format!(
            "Invalid resource_type in SQLite row: {other}"
        ))),
    }
}

// Consumed by `append_resource_audit_log_row` when it builds a
// `NewResourceAuditLog` row to insert.
pub(crate) fn event_kind_to_text(
    value: &ResourceAuditEventKind,
) -> &'static str {
    match value {
        ResourceAuditEventKind::Created => "created",
        ResourceAuditEventKind::Updated => "updated",
        ResourceAuditEventKind::Deleted => "deleted",
    }
}

fn event_kind_from_text(
    value: &str,
) -> Result<ResourceAuditEventKind, MappedErrors> {
    match value {
        "created" => Ok(ResourceAuditEventKind::Created),
        "updated" => Ok(ResourceAuditEventKind::Updated),
        "deleted" => Ok(ResourceAuditEventKind::Deleted),
        other => Err(dto_err(format!("Invalid event in SQLite row: {other}"))),
    }
}

pub(crate) fn map_model_to_dto(
    model: ResourceAuditLogModel,
) -> Result<ResourceAuditLog, MappedErrors> {
    let performed_by_value = json_from_text(&model.performed_by)?;
    let performed_by: WrittenBy = serde_json::from_value(performed_by_value)
        .map_err(|e| {
            dto_err(format!("Invalid performed_by in SQLite row: {e}"))
        })?;

    let tenant_id = model
        .tenant_id
        .map(|value| uuid_from_text(&value))
        .transpose()?;

    Ok(ResourceAuditLog {
        id: uuid_from_text(&model.id)?,
        resource_type: resource_type_from_text(&model.resource_type)?,
        resource_id: uuid_from_text(&model.resource_id)?,
        tenant_id,
        event: event_kind_from_text(&model.event)?,
        performed_by,
        metadata: json_from_text(&model.metadata)?,
        created_at: timestamp_from_text(&model.created_at)?,
    })
}
