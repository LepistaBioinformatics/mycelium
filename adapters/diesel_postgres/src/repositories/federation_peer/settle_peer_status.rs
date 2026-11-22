use super::map_peer_row::map_peer_row_to_dto;
use crate::{
    models::federation_peer::FederationPeerRow,
    schema::federation_peer as peer_model,
};

use chrono::Utc;
use diesel::{prelude::*, PgConnection};
use myc_core::domain::dtos::federation::{FederationPeer, PeerStatus};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use uuid::Uuid;

/// Statuses only an operator (or a mismatched handshake) sets and lifts
const HELD: [&str; 2] = ["quarantined", "untrusted"];

/// The status a handshake reached, written in one conditional `UPDATE` so a
/// quarantine applied while the handshake ran is never overwritten
pub(super) fn settle_peer_status(
    conn: &mut PgConnection,
    peer_id: Uuid,
    status: PeerStatus,
) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
    let settled = diesel::update(
        peer_model::table
            .find(peer_id)
            .filter(peer_model::status.ne_all(HELD)),
    )
    .set((
        peer_model::status.eq(status.as_str()),
        peer_model::updated.eq(Some(Utc::now())),
    ))
    .returning(FederationPeerRow::as_returning())
    .get_result::<FederationPeerRow>(conn)
    .optional()
    .map_err(|e| updating_err(format!("Failed to settle peer status: {e}")))?;

    match settled {
        Some(row) => {
            Ok(UpdatingResponseKind::Updated(map_peer_row_to_dto(row)?))
        }
        None => held_peer(conn, peer_id),
    }
}

fn held_peer(
    conn: &mut PgConnection,
    peer_id: Uuid,
) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
    let row = peer_model::table
        .find(peer_id)
        .select(FederationPeerRow::as_select())
        .first::<FederationPeerRow>(conn)
        .map_err(|e| updating_err(format!("peer {peer_id} not found: {e}")))?;
    let peer = map_peer_row_to_dto(row)?;
    let reason = format!("peer is {}", peer.status);

    Ok(UpdatingResponseKind::NotUpdated(peer, reason))
}
