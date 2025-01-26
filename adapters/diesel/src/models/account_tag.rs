use super::account::Account;

use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Identifiable, Associations, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::account_tag)]
#[diesel(belongs_to(Account))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct AccountTag {
    pub id: String,
    pub value: String,
    pub meta: Option<JsonValue>,
    pub account_id: String,
}
