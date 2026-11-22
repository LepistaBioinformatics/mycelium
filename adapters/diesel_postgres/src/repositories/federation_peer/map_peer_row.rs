use crate::{
    models::federation_peer::{FederationPeerKeyRow, FederationPeerRow},
    repositories::parse_optional_written_by,
};

use chrono::Local;
use myc_core::domain::dtos::federation::{
    FederationPeer, FederationPolicy, PeerKey, PeerStatus,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use serde_json::Value as JsonValue;

fn kinds(value: JsonValue) -> Result<Vec<String>, MappedErrors> {
    serde_json::from_value(value)
        .map_err(|e| fetching_err(format!("Invalid stored peer kinds: {e}")))
}

pub(super) fn map_peer_row_to_dto(
    row: FederationPeerRow,
) -> Result<FederationPeer, MappedErrors> {
    Ok(FederationPeer {
        id: row.id,
        instance_id: row.instance_id,
        base_url: row.base_url,
        display_name: row.display_name,
        status: status(&row.status)?,
        consume_kinds: kinds(row.consume_kinds)?,
        serve_kinds: kinds(row.serve_kinds)?,
        negotiated_kinds: kinds(row.negotiated_kinds)?,
        policy: policy(row.policy)?,
        last_contact_at: row.last_contact_at.map(|at| at.with_timezone(&Local)),
        failure_count: u32::try_from(row.failure_count).unwrap_or(0),
        created: row.created.with_timezone(&Local),
        created_by: parse_optional_written_by(row.created_by),
        updated: row.updated.map(|at| at.with_timezone(&Local)),
        updated_by: parse_optional_written_by(row.updated_by),
    })
}

fn status(value: &str) -> Result<PeerStatus, MappedErrors> {
    value
        .parse::<PeerStatus>()
        .map_err(|e| fetching_err(format!("Invalid stored peer: {e}")))
}

fn policy(value: JsonValue) -> Result<FederationPolicy, MappedErrors> {
    serde_json::from_value(value)
        .map_err(|e| fetching_err(format!("Invalid stored peer policy: {e}")))
}

pub(super) fn map_key_row_to_dto(row: FederationPeerKeyRow) -> PeerKey {
    PeerKey {
        peer_id: row.peer_id,
        kid: row.kid,
        alg: row.alg,
        public_key: row.public_key,
        fetched_at: row.fetched_at.with_timezone(&Local),
        expires_at: row.expires_at.map(|at| at.with_timezone(&Local)),
    }
}
