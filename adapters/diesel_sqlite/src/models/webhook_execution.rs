use diesel::prelude::*;

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::webhook_execution)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct WebHookExecution {
    pub id: String,
    pub trigger: String,
    pub payload: String,
    pub payload_id: String,
    pub created: String,
    pub status: Option<String>,
    pub attempts: i32,
    pub attempted: Option<String>,
    pub propagations: Option<String>,
    pub encrypted: Option<bool>,
}
