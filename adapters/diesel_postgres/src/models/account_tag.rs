use super::account::Account;

use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Identifiable, Associations, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::account_tag)]
#[diesel(belongs_to(Account))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct AccountTag {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    pub value: String,
    pub meta: Option<JsonValue>,
    pub account_id: Uuid,
}
