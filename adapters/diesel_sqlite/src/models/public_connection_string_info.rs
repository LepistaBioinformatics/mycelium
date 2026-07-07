use diesel::prelude::*;
use diesel::sql_types::{Integer, Nullable, Text};

/// Mirrors the `public_connection_string_info` SQLite view. Unlike postgres
/// (where every projected column is native `Jsonb`), SQLite's `json_extract`
/// unwraps scalar JSON values (strings) to plain TEXT and only re-serializes
/// compound values (objects/arrays) as JSON text -- so `email`/`scope` need
/// JSON parsing on read, while `innerId`/`accountId`/`name`/`createdAt` are
/// already plain strings.
#[derive(Clone, Debug, QueryableByName)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PublicConnectionStringInfoModel {
    #[diesel(sql_type = Integer, column_name = "id")]
    pub id: i32,
    #[diesel(sql_type = Nullable<Text>, column_name = "innerId")]
    pub inner_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>, column_name = "accountId")]
    pub account_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>, column_name = "email")]
    pub email: Option<String>,
    #[diesel(sql_type = Nullable<Text>, column_name = "name")]
    pub name: Option<String>,
    #[diesel(sql_type = Text, column_name = "expiration")]
    pub expiration: String,
    #[diesel(sql_type = Nullable<Text>, column_name = "createdAt")]
    pub created_at: Option<String>,
    #[diesel(sql_type = Nullable<Text>, column_name = "scope")]
    pub scope: Option<String>,
}
