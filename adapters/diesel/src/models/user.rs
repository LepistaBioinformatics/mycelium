use super::account::Account;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(
    Identifiable, Associations, Debug, Queryable, Selectable, Insertable,
)]
#[diesel(table_name = crate::schema::user)]
#[diesel(belongs_to(Account))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    #[diesel(sql_type = Option<diesel::sql_types::Uuid>)]
    pub account_id: Option<Uuid>,
    pub is_active: bool,
    pub is_principal: bool,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
    pub mfa: Option<JsonValue>,
}
