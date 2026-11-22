use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::federation_identity_link)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct IdentityLinkRow {
    pub id: Uuid,
    pub local_user_id: Uuid,
    pub local_email: String,
    pub peer_id: Uuid,
    pub peer_subject_ref: Option<String>,
    pub local_subject_ref: String,
    pub status: String,
    pub created: DateTime<Utc>,
    pub established_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by: Option<JsonValue>,
}

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::federation_link_request)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct LinkRequestRow {
    pub id: Uuid,
    pub local_user_id: Uuid,
    pub peer_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub redirected_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::federation_link_consent)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct LinkConsentRow {
    pub id: Uuid,
    pub peer_id: Uuid,
    pub peer_request_id: Uuid,
    pub peer_email: String,
    pub return_url: String,
    pub local_user_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
}
