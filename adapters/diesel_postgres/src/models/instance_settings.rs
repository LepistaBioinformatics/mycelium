use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::instance_settings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct InstanceSettingsRow {
    pub key: String,
    pub value: JsonValue,
    pub created_by: Option<JsonValue>,
    pub updated_by: Option<JsonValue>,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
}
