// ? ---------------------------------------------------------------------------
// ? append_resource_audit_log_row
//
// Inserts a single `NewResourceAuditLogEvent` as a `resource_audit_log` row.
// Mirrors `mycelium-diesel-postgres`'s `append_resource_audit_log_row` --
// same signature shape and error handling -- but targets this backend's own
// `SqliteConnection`, `Insertable` model, and text-encoding functions. Lives
// here -- not in `ports/api` -- so the Diesel schema, the `Insertable` model,
// and the resource-type/event text encoding stay private to this adapter.
//
// Unlike the Postgres table, `resource_audit_log.id` has no DB-side default
// in SQLite (see the migration's header comment), so this helper generates
// the row's UUID itself, the same way every other SQLite registration
// repository in this crate does.
// ? ---------------------------------------------------------------------------

use super::shared::{event_kind_to_text, resource_type_to_text};
use crate::{
    models::resource_audit_log::ResourceAuditLog as NewResourceAuditLog,
    schema::resource_audit_log as resource_audit_log_model,
    types::{json_to_text, timestamp_to_text, uuid_to_text},
};

use diesel::prelude::*;
use diesel::SqliteConnection;
use myc_core::domain::dtos::resource_audit_log::NewResourceAuditLogEvent;
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use uuid::Uuid;

pub fn append_resource_audit_log_row(
    conn: &mut SqliteConnection,
    event: &NewResourceAuditLogEvent,
) -> Result<(), MappedErrors> {
    let performed_by =
        json_to_text(&serde_json::to_value(&event.performed_by).map_err(
            |e| creation_err(format!("Failed to encode performed_by: {e}")),
        )?)?;

    let new_row = NewResourceAuditLog {
        id: uuid_to_text(&Uuid::new_v4()),
        resource_type: resource_type_to_text(&event.resource_type).to_owned(),
        resource_id: uuid_to_text(&event.resource_id),
        tenant_id: event.tenant_id.map(|id| uuid_to_text(&id)),
        event: event_kind_to_text(&event.event).to_owned(),
        performed_by,
        metadata: json_to_text(&event.metadata)?,
        created_at: timestamp_to_text(&event.created_at),
    };

    diesel::insert_into(resource_audit_log_model::table)
        .values(&new_row)
        .execute(conn)
        .map_err(|e| {
            creation_err(format!("Failed to insert resource audit log: {e}"))
        })?;

    Ok(())
}
