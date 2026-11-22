use super::{map_peer_row::map_peer_row_to_dto, peer_json::to_json};
use crate::{
    models::federation_peer::FederationPeerRow,
    schema::federation_peer as peer_model,
};

use diesel::{prelude::*, PgConnection};
use myc_core::domain::dtos::{
    federation::{FederationPeer, NewFederationPeer, PeerStatus},
    written_by::WrittenBy,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};

/// Insert the peer; an existing `instance_id` is a conflict, never a
/// duplicate row (FED-07)
pub(super) fn declare_peer(
    conn: &mut PgConnection,
    peer: NewFederationPeer,
    created_by: Option<WrittenBy>,
) -> Result<CreateResponseKind<FederationPeer>, MappedErrors> {
    let Some(row) = insert_peer(conn, &peer, created_by)? else {
        return already_declared(conn, peer);
    };

    Ok(CreateResponseKind::Created(map_peer_row_to_dto(row)?))
}

/// `None` when a peer with the same `instance_id` already exists
fn insert_peer(
    conn: &mut PgConnection,
    peer: &NewFederationPeer,
    created_by: Option<WrittenBy>,
) -> Result<Option<FederationPeerRow>, MappedErrors> {
    diesel::insert_into(peer_model::table)
        .values((
            peer_model::instance_id.eq(peer.instance_id),
            peer_model::base_url.eq(&peer.base_url),
            peer_model::display_name.eq(&peer.display_name),
            peer_model::status.eq(PeerStatus::Declared.as_str()),
            peer_model::consume_kinds.eq(to_json(&peer.consume_kinds)?),
            peer_model::serve_kinds.eq(to_json(&peer.serve_kinds)?),
            peer_model::policy.eq(to_json(&peer.policy)?),
            peer_model::created_by
                .eq(created_by.as_ref().map(to_json).transpose()?),
        ))
        .on_conflict(peer_model::instance_id)
        .do_nothing()
        .returning(FederationPeerRow::as_returning())
        .get_result::<FederationPeerRow>(conn)
        .optional()
        .map_err(|e| creation_err(format!("Failed to declare peer: {e}")))
}

fn already_declared(
    conn: &mut PgConnection,
    peer: NewFederationPeer,
) -> Result<CreateResponseKind<FederationPeer>, MappedErrors> {
    let existing = peer_model::table
        .filter(peer_model::instance_id.eq(peer.instance_id))
        .select(FederationPeerRow::as_select())
        .first::<FederationPeerRow>(conn)
        .map_err(|e| creation_err(format!("Failed to fetch peer: {e}")))?;

    Ok(CreateResponseKind::NotCreated(
        map_peer_row_to_dto(existing)?,
        format!(
            "a peer with instance id {} is already declared",
            peer.instance_id
        ),
    ))
}
