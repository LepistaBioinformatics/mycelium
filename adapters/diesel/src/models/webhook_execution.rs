use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::webhook_execution)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct WebHookExecution {
    pub id: String,
    pub correspondence_id: String,
    pub trigger: String,
    pub artifact: String,
    pub created: NaiveDateTime,
    pub execution_details: Option<JsonValue>,
}
