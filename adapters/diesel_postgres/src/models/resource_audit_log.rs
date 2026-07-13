use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::resource_audit_log)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct ResourceAuditLog {
    pub id: Uuid,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub event: String,
    pub performed_by: JsonValue,
    pub metadata: JsonValue,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::resource_audit_log)]
pub(crate) struct NewResourceAuditLog {
    pub resource_type: String,
    pub resource_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub event: String,
    pub performed_by: JsonValue,
    pub metadata: JsonValue,
    pub created_at: NaiveDateTime,
}
