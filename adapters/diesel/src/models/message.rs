use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::message_queue)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct Message {
    pub id: Uuid,
    pub message: String,
    pub created: NaiveDateTime,
    pub attempted: Option<NaiveDateTime>,
    pub status: String,
    pub attempts: i32,
    pub error: Option<String>,
}
