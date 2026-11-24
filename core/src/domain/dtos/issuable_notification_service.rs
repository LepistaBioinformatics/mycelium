use super::service_notification_config::RecipientPolicy;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A service the caller can issue notification connection strings for.
///
/// Carries only what an issuer needs to fill the issuance form; the service
/// issuer roles, templates, quotas and allowed domains stay private.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IssuableNotificationService {
    /// The service senderTag, sent back on issuance
    pub sender_tag: String,

    /// The service display name
    pub display_name: String,

    /// Who the service may email
    pub recipient_policy: RecipientPolicy,

    /// Tenants the caller can bind a string of this service to
    pub tenant_ids: Vec<Uuid>,

    /// Whether a string can be issued without a tenant
    pub allows_no_tenant: bool,
}
