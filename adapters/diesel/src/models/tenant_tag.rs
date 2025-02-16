use super::tenant::Tenant;

use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(
    Identifiable, Associations, Clone, Debug, Queryable, Insertable, Selectable,
)]
#[diesel(table_name = crate::schema::tenant_tag)]
#[diesel(belongs_to(Tenant))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct TenantTag {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    pub value: String,
    pub meta: Option<JsonValue>,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub tenant_id: Uuid,
}
