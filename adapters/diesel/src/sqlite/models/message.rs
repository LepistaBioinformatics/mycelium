use diesel::prelude::*;

/// `created`/`attempted` are a genuine timezone-aware round trip (mirrors the
/// postgres repo, which converts via `Local.from_utc_datetime(&naive)` rather
/// than the naive-reinterpretation convention) -- see
/// `sqlite::types::{timestamp_to_text, timestamp_from_text}`.
#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::sqlite::schema::message_queue)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct Message {
    pub id: String,
    pub message: String,
    pub created: String,
    pub attempted: Option<String>,
    pub status: String,
    pub attempts: i32,
    pub error: Option<String>,
}
