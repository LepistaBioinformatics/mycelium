use super::account::Account;
use super::guest_user::GuestUser;

use diesel::prelude::*;

#[derive(
    Identifiable, Associations, Debug, Queryable, Selectable, Insertable,
)]
#[diesel(table_name = crate::sqlite::schema::guest_user_on_account)]
#[diesel(belongs_to(Account, foreign_key = account_id))]
#[diesel(belongs_to(GuestUser, foreign_key = guest_user_id))]
#[diesel(primary_key(guest_user_id, account_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GuestUserOnAccount {
    pub guest_user_id: String,
    pub account_id: String,
    pub created: String,
    pub permit_flags: Option<String>,
    pub deny_flags: Option<String>,
}
