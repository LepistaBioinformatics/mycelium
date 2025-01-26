use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Identifiable, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::account)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct Account {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub meta: Option<JsonValue>,
    pub account_type: JsonValue,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
    pub is_active: bool,
    pub is_checked: bool,
    pub is_archived: bool,
    pub is_default: bool,
    pub tenant_id: Option<String>,
}
