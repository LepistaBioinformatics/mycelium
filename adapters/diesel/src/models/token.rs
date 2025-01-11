use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::token)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct Token {
    pub id: i32,
    pub meta: JsonValue,
    pub expiration: NaiveDateTime,
} 