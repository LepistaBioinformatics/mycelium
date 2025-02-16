use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Identifiable, Clone, Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::tenant)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct Tenant {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub meta: Option<JsonValue>,
    pub status: Option<Vec<JsonValue>>,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
}
