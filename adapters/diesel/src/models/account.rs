use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Identifiable, Clone, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::account)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct Account {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub meta: Option<JsonValue>,
    pub account_type: JsonValue,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
    pub is_active: bool,
    pub is_checked: bool,
    pub is_archived: bool,
    pub is_default: bool,
    #[diesel(sql_type = Option<diesel::sql_types::Uuid>)]
    pub tenant_id: Option<Uuid>,
}
