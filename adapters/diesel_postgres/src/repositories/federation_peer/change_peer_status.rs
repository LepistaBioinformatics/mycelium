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

pub(super) fn change_peer_status(
    conn: &mut PgConnection,
    peer_id: Uuid,
    status: PeerStatus,
) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors> {
    let row = diesel::update(peer_model::table.find(peer_id))
        .set((
            peer_model::status.eq(status.as_str()),
            peer_model::updated.eq(Some(Utc::now())),
        ))
        .returning(FederationPeerRow::as_returning())
        .get_result::<FederationPeerRow>(conn)
        .optional()
        .map_err(|e| {
            updating_err(format!("Failed to update peer status: {e}"))
        })?;

    let Some(row) = row else {
        return updating_err(format!("peer {peer_id} not found"))
            .with_exp_true()
            .as_error();
    };

    Ok(UpdatingResponseKind::Updated(map_peer_row_to_dto(row)?))
}
