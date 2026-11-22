use crate::domain::{
    dtos::{
        federation::{PeerOutcome, PeerResolution},
        resource_audit_log::{
            NewResourceAuditLogEvent, ResourceAuditEventKind,
            ResourceAuditResourceType,
        },
        written_by::WrittenBy,
    },
    entities::ResourceAuditLogRegistration,
};

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

/// Record one refresh of a link's grants (FED-16)
///
/// Called for every peer actually asked, never for reads from the store. A
/// failure to write the audit row is logged; it does not undo the refresh.
///
#[tracing::instrument(name = "audit_peer_resolution", skip_all)]
pub async fn audit_peer_resolution(
    link_id: Uuid,
    outcome: &PeerOutcome,
    author: &WrittenBy,
    audit_repo: Box<&dyn ResourceAuditLogRegistration>,
) {
    let event = NewResourceAuditLogEvent {
        resource_type: ResourceAuditResourceType::FederationResolution,
        resource_id: link_id,
        tenant_id: None,
        event: ResourceAuditEventKind::Updated,
        performed_by: author.clone(),
        metadata: metadata_of(outcome),
        created_at: Utc::now(),
    };

    if let Err(err) = audit_repo.create(event).await {
        tracing::error!(%err, %link_id, "federated resolution not audited");
    }
}

fn metadata_of(outcome: &PeerOutcome) -> serde_json::Value {
    let accepted = match outcome.resolution {
        PeerResolution::Answered => outcome.grants.len(),
        _ => 0,
    };

    json!({
        "peerId": outcome.peer_id,
        "peerInstanceId": outcome.instance_id,
        "resolution": outcome.resolution,
        "grantsAccepted": accepted,
    })
}
