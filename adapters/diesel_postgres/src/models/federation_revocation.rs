use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::federation_outbox)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct OutboxRow {
    pub id: Uuid,
    pub peer_id: Uuid,
    pub subject_ref: String,
    pub reason: String,
    pub issued_at: DateTime<Utc>,
    pub attempts: i32,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = crate::schema::federation_outbox)]
pub(crate) struct NewOutboxRow {
    pub peer_id: Uuid,
    pub subject_ref: String,
    pub reason: String,
    pub issued_at: DateTime<Utc>,
    pub next_attempt_at: DateTime<Utc>,
}
