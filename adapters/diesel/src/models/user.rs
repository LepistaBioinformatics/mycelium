use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    #[diesel(serialize_as = String)]
    #[diesel(deserialize_as = String)]
    pub id: String,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub account_id: Option<String>,
    pub is_active: bool,
    pub is_principal: bool,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
    pub mfa: Option<JsonValue>,
}
