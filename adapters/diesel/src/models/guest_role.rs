use diesel::prelude::*;
use uuid::Uuid;

#[derive(Identifiable, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::guest_role)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuestRole {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permission: i32,
    pub slug: String,
}
