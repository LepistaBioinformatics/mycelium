use super::tenant::Tenant;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(
    Identifiable, Associations, Clone, Debug, Queryable, Selectable, Insertable,
)]
#[diesel(table_name = crate::schema::owner_on_tenant)]
#[diesel(belongs_to(Tenant, foreign_key = tenant_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OwnerOnTenant {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub tenant_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub owner_id: Uuid,
    pub guest_by: String,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
}
