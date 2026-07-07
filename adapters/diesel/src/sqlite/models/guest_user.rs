use super::guest_role::GuestRole;

use diesel::prelude::*;

/// Unlike most sqlite models, `created`/`updated` here mirror the postgres
/// model's genuine timezone-aware round trip (`DateTime<Local>`, not
/// `NaiveDateTime` + reinterpretation) -- see `sqlite::types::{timestamp_to_text,
/// timestamp_from_text}`.
#[derive(
    Identifiable, Associations, Debug, Queryable, Insertable, Selectable,
)]
#[diesel(table_name = crate::sqlite::schema::guest_user)]
#[diesel(belongs_to(GuestRole))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct GuestUser {
    pub id: String,
    pub email: String,
    pub guest_role_id: String,
    pub created: String,
    pub updated: Option<String>,
    pub was_verified: bool,
}
