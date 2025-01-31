use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::webhook_execution)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct WebHookExecution {
    pub id: String,
    pub trigger: String,
    pub payload: String,
    pub created: NaiveDateTime,
    pub status: Option<String>,
    pub attempts: i32,
    pub attempted: Option<NaiveDateTime>,
    pub propagations: Option<JsonValue>,
    pub encrypted: Option<bool>,
}
