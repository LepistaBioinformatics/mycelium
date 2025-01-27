use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::webhook)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct WebHook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub trigger: String,
    pub secret: Option<JsonValue>,
    pub is_active: bool,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
}
