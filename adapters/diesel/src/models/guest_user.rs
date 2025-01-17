use chrono::{DateTime, Local};
use diesel::prelude::*;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::guest_user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct GuestUser {
    pub id: String,
    pub email: String,
    pub guest_role_id: String,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
    pub was_verified: bool,
}
