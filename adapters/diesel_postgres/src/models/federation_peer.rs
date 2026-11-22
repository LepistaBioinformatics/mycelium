use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::federation_peer)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct FederationPeerRow {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub base_url: String,
    pub display_name: Option<String>,
    pub status: String,
    pub consume_kinds: JsonValue,
    pub serve_kinds: JsonValue,
    pub negotiated_kinds: JsonValue,
    pub policy: JsonValue,
    pub last_contact_at: Option<DateTime<Utc>>,
    pub failure_count: i32,
    pub created: DateTime<Utc>,
    pub created_by: Option<JsonValue>,
    pub updated: Option<DateTime<Utc>>,
    pub updated_by: Option<JsonValue>,
}

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::federation_peer_key)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct FederationPeerKeyRow {
    pub peer_id: Uuid,
    pub kid: String,
    pub alg: String,
    pub public_key: String,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
