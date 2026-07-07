use super::tenant::Tenant;

use diesel::prelude::*;

#[derive(
    Identifiable, Associations, Clone, Debug, Queryable, Insertable, Selectable,
)]
#[diesel(table_name = crate::sqlite::schema::tenant_tag)]
#[diesel(belongs_to(Tenant))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct TenantTag {
    pub id: String,
    pub value: String,
    pub meta: Option<String>,
    pub tenant_id: String,
}
