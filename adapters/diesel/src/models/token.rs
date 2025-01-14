use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Jsonb, Timestamptz};
use serde_json::Value as JsonValue;

#[derive(QueryableByName, Queryable, Selectable)]
#[diesel(table_name = crate::schema::token)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Token {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Timestamptz)]
    pub expiration: NaiveDateTime,
    #[diesel(sql_type = Jsonb)]
    pub meta: JsonValue,
}
