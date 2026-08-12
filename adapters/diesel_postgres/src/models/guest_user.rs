use super::guest_role::GuestRole;
use chrono::{DateTime, Local};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(
    Identifiable, Associations, Debug, Queryable, Insertable, Selectable,
)]
#[diesel(table_name = crate::schema::guest_user)]
#[diesel(belongs_to(GuestRole))]
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
