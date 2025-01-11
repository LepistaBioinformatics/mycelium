use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::owner_on_tenant)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct OwnerOnTenant {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub owner_id: Uuid,
    pub guest_by: String,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
}
