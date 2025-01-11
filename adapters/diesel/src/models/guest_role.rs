use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::guest_role)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct GuestRole {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub permission: i32,
} 