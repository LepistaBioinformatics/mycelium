use diesel::prelude::*;

#[derive(Identifiable, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::guest_role)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuestRole {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub permission: i32,
    pub slug: String,
}
