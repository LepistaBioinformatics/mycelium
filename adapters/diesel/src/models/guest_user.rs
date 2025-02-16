use chrono::{DateTime, Local};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Identifiable, Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::guest_user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct GuestUser {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    pub email: String,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub guest_role_id: Uuid,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
    pub was_verified: bool,
}
