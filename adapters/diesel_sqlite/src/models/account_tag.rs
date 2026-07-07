use super::account::Account;

use diesel::prelude::*;

#[derive(Identifiable, Associations, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::account_tag)]
#[diesel(belongs_to(Account))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct AccountTag {
    pub id: String,
    pub value: String,
    pub meta: Option<String>,
    pub account_id: String,
}
