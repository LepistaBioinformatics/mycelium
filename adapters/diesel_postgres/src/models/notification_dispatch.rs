use chrono::{DateTime, Utc};
use diesel::prelude::*;
use myc_core::domain::dtos::notification_dispatch::NewNotificationDispatch;
use uuid::Uuid;

/// One `notification_dispatch` row. `channel` is left to its column default
/// (`email`).
#[derive(Insertable)]
#[diesel(table_name = crate::schema::notification_dispatch)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct NewNotificationDispatchRow {
    pub id: Uuid,
    pub sender_tag: String,
    pub connection_string_id: i32,
    pub account_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub recipient: String,
    pub template: String,
    pub locale: String,
    pub created_at: DateTime<Utc>,
}

impl NewNotificationDispatchRow {
    pub(crate) fn new(
        sender_tag: &str,
        dispatch: &NewNotificationDispatch,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: dispatch.id,
            sender_tag: sender_tag.to_owned(),
            connection_string_id: dispatch.issuer.connection_string_id,
            account_id: dispatch.issuer.account_id,
            tenant_id: dispatch.issuer.tenant_id,
            recipient: dispatch.recipient.email(),
            template: dispatch.template.to_owned(),
            locale: dispatch.locale.to_owned(),
            created_at,
        }
    }
}
