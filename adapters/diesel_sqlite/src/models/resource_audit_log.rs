use diesel::prelude::*;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::resource_audit_log)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ResourceAuditLog {
    pub id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub tenant_id: Option<String>,
    pub event: String,
    pub performed_by: String,
    pub metadata: String,
    pub created_at: String,
}
