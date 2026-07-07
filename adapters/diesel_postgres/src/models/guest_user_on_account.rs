use super::account::Account;
use super::guest_user::GuestUser;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Identifiable, Associations, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::guest_user_on_account)]
#[diesel(belongs_to(Account, foreign_key = account_id))]
#[diesel(belongs_to(GuestUser, foreign_key = guest_user_id))]
#[diesel(primary_key(guest_user_id, account_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuestUserOnAccount {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub guest_user_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub account_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created: NaiveDateTime,
    #[diesel(sql_type = Nullable<Array<Text>>)]
    pub permit_flags: Option<Vec<String>>,
    #[diesel(sql_type = Nullable<Array<Text>>)]
    pub deny_flags: Option<Vec<String>>,
}
