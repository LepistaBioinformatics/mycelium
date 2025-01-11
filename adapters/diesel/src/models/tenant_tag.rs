use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::tenant_tag)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct TenantTag {
    pub id: Uuid,
    pub value: String,
    pub meta: Option<JsonValue>,
    pub tenant_id: Uuid,
} 