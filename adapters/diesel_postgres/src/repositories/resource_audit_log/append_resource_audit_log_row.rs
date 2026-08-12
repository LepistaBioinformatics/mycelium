// ? ---------------------------------------------------------------------------
// ? append_resource_audit_log_row
//
// Inserts a single `NewResourceAuditLogEvent` as a `resource_audit_log` row.
// Lives here -- not in `ports/api` -- so the Diesel schema, the `Insertable`
// model, and the resource-type/event-kind DB-string encoding stay private to
// this adapter. The `resource_audit_log_dispatcher` (ports/api) only needs a
// pooled connection (via `DbPoolProvider`) and this one function; it never
// touches Diesel directly, matching every other cross-boundary call in this
// codebase.
// ? ---------------------------------------------------------------------------

use super::resource_audit_log_db_encoding::{
    event_kind_to_db_str, resource_type_to_db_str,
};
use crate::{
    models::resource_audit_log::NewResourceAuditLog,
    schema::resource_audit_log as resource_audit_log_model,
};

use diesel::prelude::*;
use diesel::PgConnection;
use myc_core::domain::dtos::resource_audit_log::NewResourceAuditLogEvent;
use mycelium_base::utils::errors::{creation_err, MappedErrors};

pub fn append_resource_audit_log_row(
    conn: &mut PgConnection,
    event: &NewResourceAuditLogEvent,
) -> Result<(), MappedErrors> {
    let performed_by =
        serde_json::to_value(&event.performed_by).map_err(|e| {
            creation_err(format!("Failed to encode performed_by: {e}"))
        })?;

    let new_row = NewResourceAuditLog {
        resource_type: resource_type_to_db_str(&event.resource_type).to_owned(),
        resource_id: event.resource_id,
        tenant_id: event.tenant_id,
        event: event_kind_to_db_str(&event.event).to_owned(),
        performed_by,
        metadata: event.metadata.to_owned(),
        created_at: event.created_at.naive_utc(),
    };

    diesel::insert_into(resource_audit_log_model::table)
        .values(&new_row)
        .execute(conn)
        .map_err(|e| {
            creation_err(format!("Failed to insert resource audit log: {e}"))
        })?;

    Ok(())
}
