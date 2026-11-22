use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::federation_assertion_cache)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct AssertionRow {
    pub link_id: Uuid,
    pub peer_id: Uuid,
    pub kind: String,
    pub jws: String,
    pub payload: JsonValue,
    pub iat: DateTime<Utc>,
    pub exp: DateTime<Utc>,
}
