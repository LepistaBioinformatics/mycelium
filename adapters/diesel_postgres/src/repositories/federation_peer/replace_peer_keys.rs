use crate::schema::federation_peer_key as key_model;

use chrono::{DateTime, Utc};
use diesel::{prelude::*, PgConnection};
use myc_core::domain::dtos::federation::NewPeerKey;
use mycelium_base::utils::errors::{updating_err, MappedErrors};
use uuid::Uuid;

/// Replace every stored key of the peer, atomically
pub(super) fn replace_peer_keys(
    conn: &mut PgConnection,
    peer_id: Uuid,
    keys: &[NewPeerKey],
) -> Result<(), MappedErrors> {
    let rows = key_rows(peer_id, keys);

    conn.transaction(|conn| {
        diesel::delete(key_model::table.filter(key_model::peer_id.eq(peer_id)))
            .execute(conn)?;
        diesel::insert_into(key_model::table)
            .values(&rows)
            .execute(conn)
    })
    .map(|_| ())
    .map_err(|e| updating_err(format!("Failed to record peer keys: {e}")))
}

type KeyRow<'a> = (
    diesel::dsl::Eq<key_model::peer_id, Uuid>,
    diesel::dsl::Eq<key_model::kid, &'a String>,
    diesel::dsl::Eq<key_model::alg, &'a String>,
    diesel::dsl::Eq<key_model::public_key, &'a String>,
    diesel::dsl::Eq<key_model::expires_at, Option<DateTime<Utc>>>,
);

fn key_rows(peer_id: Uuid, keys: &[NewPeerKey]) -> Vec<KeyRow<'_>> {
    keys.iter()
        .map(|key| {
            (
                key_model::peer_id.eq(peer_id),
                key_model::kid.eq(&key.kid),
                key_model::alg.eq(&key.alg),
                key_model::public_key.eq(&key.public_key),
                key_model::expires_at
                    .eq(key.expires_at.map(|at| at.with_timezone(&Utc))),
            )
        })
        .collect()
}
