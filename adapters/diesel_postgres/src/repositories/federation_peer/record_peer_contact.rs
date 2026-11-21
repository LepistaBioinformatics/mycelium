use super::peer_json::to_json;
use crate::schema::federation_peer as peer_model;

use chrono::Utc;
use diesel::{prelude::*, PgConnection};
use mycelium_base::utils::errors::{updating_err, MappedErrors};
use uuid::Uuid;

/// A successful exchange: the failure count resets
pub(super) fn record_contact_success(
    conn: &mut PgConnection,
    peer_id: Uuid,
) -> Result<(), MappedErrors> {
    diesel::update(peer_model::table.find(peer_id))
        .set((
            peer_model::failure_count.eq(0),
            peer_model::last_contact_at.eq(Some(Utc::now())),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| {
            updating_err(format!("Failed to record peer success: {e}"))
        })
}

/// One more consecutive failure; returns the new count
pub(super) fn record_contact_failure(
    conn: &mut PgConnection,
    peer_id: Uuid,
) -> Result<u32, MappedErrors> {
    let count = diesel::update(peer_model::table.find(peer_id))
        .set(peer_model::failure_count.eq(peer_model::failure_count + 1))
        .returning(peer_model::failure_count)
        .get_result::<i32>(conn)
        .map_err(|e| {
            updating_err(format!("Failed to record peer failure: {e}"))
        })?;

    Ok(u32::try_from(count).unwrap_or(0))
}

/// The assertion kinds both sides support, set by a handshake
pub(super) fn record_negotiated_kinds(
    conn: &mut PgConnection,
    peer_id: Uuid,
    kinds: &[String],
) -> Result<(), MappedErrors> {
    diesel::update(peer_model::table.find(peer_id))
        .set((
            peer_model::negotiated_kinds.eq(to_json(&kinds)?),
            peer_model::updated.eq(Some(Utc::now())),
        ))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| {
            updating_err(format!("Failed to set negotiated kinds: {e}"))
        })
}
