use super::tenant::Tenant;

use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(
    Identifiable, Associations, Clone, Debug, Queryable, Selectable, Insertable,
)]
#[diesel(table_name = crate::schema::owner_on_tenant)]
#[diesel(belongs_to(Tenant, foreign_key = tenant_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OwnerOnTenant {
    pub id: String,
    pub tenant_id: String,
    pub owner_id: String,
    pub guest_by: String,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
}
