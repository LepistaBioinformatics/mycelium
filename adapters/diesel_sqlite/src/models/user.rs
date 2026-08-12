use super::account::Account;

use diesel::prelude::*;

#[derive(
    Identifiable, Associations, Debug, Queryable, Selectable, Insertable,
)]
#[diesel(table_name = crate::schema::user)]
#[diesel(belongs_to(Account))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub is_active: bool,
    pub created: String,
    pub updated: Option<String>,
    pub account_id: Option<String>,
    pub is_principal: bool,
    pub mfa: Option<String>,
}
