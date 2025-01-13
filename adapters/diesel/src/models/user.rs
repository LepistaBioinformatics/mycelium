use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub account_id: Option<Uuid>,
    pub is_active: bool,
    pub is_principal: bool,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
    pub mfa: Option<JsonValue>,
}
