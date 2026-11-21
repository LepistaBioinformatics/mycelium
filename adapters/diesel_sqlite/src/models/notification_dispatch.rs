use diesel::prelude::*;

/// One `notification_dispatch` row. `channel` is left to its column default
/// (`email`). `created_at` uses the fixed-width form of
/// `dispatch_timestamp_to_text`, so it can be compared as text.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::notification_dispatch)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct NewNotificationDispatchRow {
    pub id: String,
    pub sender_tag: String,
    pub connection_string_id: i32,
    pub account_id: String,
    pub tenant_id: Option<String>,
    pub recipient: String,
    pub template: String,
    pub locale: String,
    pub created_at: String,
}
