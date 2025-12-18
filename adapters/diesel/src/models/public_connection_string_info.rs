use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Jsonb, Nullable, Timestamptz};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublicConnectionStringInfoModel {
    #[diesel(sql_type = Integer, column_name = "id")]
    pub id: i32,
    #[diesel(sql_type = Nullable<Jsonb>, column_name = "innerId")]
    pub inner_id: Option<JsonValue>,
    #[diesel(sql_type = Nullable<Jsonb>, column_name = "accountId")]
    pub account_id: Option<JsonValue>,
    #[diesel(sql_type = Nullable<Jsonb>, column_name = "email")]
    pub email: Option<JsonValue>,
    #[diesel(sql_type = Nullable<Jsonb>, column_name = "name")]
    pub name: Option<JsonValue>,
    #[diesel(sql_type = Timestamptz, column_name = "expiration")]
    pub expiration: NaiveDateTime,
    #[diesel(sql_type = Nullable<Jsonb>, column_name = "createdAt")]
    pub created_at: Option<JsonValue>,
    #[diesel(sql_type = Nullable<Jsonb>, column_name = "scope")]
    pub scope: Option<JsonValue>,
}
