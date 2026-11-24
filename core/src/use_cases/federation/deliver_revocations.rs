use super::RevocationDeliveryPorts;
use crate::domain::dtos::federation::OutboxEntry;

use chrono::{DateTime, Local};
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use std::time::Duration;

/// Attempts before a row is given up (about 4 h of backoff)
pub const MAX_DELIVERY_ATTEMPTS: u32 = 20;

/// Rows claimed at once
const CLAIM_SIZE: u32 = 50;

/// How long a claimed row is held by the pod delivering it
const CLAIM_LEASE: Duration = Duration::from_secs(60);

/// Push every due revocation once (FED-24)
///
/// A delivered row is deleted. A failed one is retried after 5 s · 2ⁿ,
/// capped at 15 min, and given up after `MAX_DELIVERY_ATTEMPTS`. A row whose
/// peer is gone is dropped: the peer's data went with it. Returns how many
/// were delivered.
///
#[tracing::instrument(name = "deliver_revocations", skip_all)]
pub async fn deliver_revocations(
    ports: RevocationDeliveryPorts<'_>,
) -> Result<u32, MappedErrors> {
    let due = ports.outbox.claim_due(CLAIM_SIZE, CLAIM_LEASE).await?;
    let mut delivered = 0;

    for entry in due {
        delivered += deliver(entry, ports).await? as u32;
    }

    Ok(delivered)
}

async fn deliver(
    entry: OutboxEntry,
    ports: RevocationDeliveryPorts<'_>,
) -> Result<bool, MappedErrors> {
    let peer_id = entry.revocation.peer_id;
    let FetchResponseKind::Found(peer) = ports.peers.get(peer_id).await? else {
        ports.outbox.delivered(entry.id).await?;
        return Ok(false);
    };

    let tombstone = entry.revocation.tombstone.clone();
    let Err(err) = ports.sender.push(&peer, tombstone).await else {
        ports.outbox.delivered(entry.id).await?;
        return Ok(true);
    };

    settle_failure(&entry, err.to_string(), ports).await?;
    Ok(false)
}

async fn settle_failure(
    entry: &OutboxEntry,
    error: String,
    ports: RevocationDeliveryPorts<'_>,
) -> Result<(), MappedErrors> {
    tracing::warn!(peer = %entry.revocation.peer_id, %error, "revocation not delivered");

    if entry.attempts + 1 >= MAX_DELIVERY_ATTEMPTS {
        return ports.outbox.give_up(entry.id, error).await;
    }

    let at = next_attempt_at(entry.attempts, Local::now());
    ports.outbox.retry(entry.id, at, error).await
}

/// 5 s · 2ⁿ after `now`, capped at 15 min
fn next_attempt_at(attempts: u32, now: DateTime<Local>) -> DateTime<Local> {
    let seconds = 5u64.saturating_mul(1 << attempts.min(16)).min(900);
    now + chrono::Duration::seconds(seconds as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::{
            PeerStatus, PendingRevocation, SubjectRef, Tombstone,
            TombstoneReason,
        },
        use_cases::federation::{
            link_test_doubles::{peer, LinkStore},
            revocation_test_doubles::{Revocations, Settled},
        },
    };

    use uuid::Uuid;

    fn entry(peer_id: Uuid, attempts: u32) -> OutboxEntry {
        OutboxEntry {
            id: Uuid::new_v4(),
            revocation: PendingRevocation {
                peer_id,
                tombstone: Tombstone {
                    subject_ref: SubjectRef::mint(),
                    reason: TombstoneReason::GrantsChanged,
                    issued_at: Local::now(),
                },
            },
            attempts,
        }
    }

    async fn deliver_all(store: &LinkStore, revocations: &Revocations) -> u32 {
        deliver_revocations(RevocationDeliveryPorts {
            outbox: revocations,
            peers: store,
            sender: revocations,
        })
        .await
        .unwrap()
    }

    #[test]
    fn the_backoff_doubles_and_is_capped() {
        let now = Local::now();
        let wait =
            |attempts| (next_attempt_at(attempts, now) - now).num_seconds();

        assert_eq!([wait(0), wait(1), wait(2)], [5, 10, 20]);
        assert_eq!(wait(19), 900);
    }

    #[tokio::test]
    async fn delivered_rows_go_and_rows_of_a_gone_peer_are_dropped() {
        let known = peer(PeerStatus::Active);
        let store = LinkStore::with_peers(vec![known.clone()]);
        let revocations = Revocations::default();
        let (sent, orphan) = (entry(known.id, 0), entry(Uuid::new_v4(), 0));
        revocations
            .due
            .lock()
            .unwrap()
            .extend([sent.clone(), orphan.clone()]);

        assert_eq!(deliver_all(&store, &revocations).await, 1);
        assert_eq!(revocations.pushed.lock().unwrap()[0].0, known.id);
        assert_eq!(
            *revocations.settled.lock().unwrap(),
            vec![Settled::Delivered(sent.id), Settled::Delivered(orphan.id)]
        );
    }

    #[tokio::test]
    async fn a_refused_push_is_retried_then_given_up() {
        let known = peer(PeerStatus::Active);
        let store = LinkStore::with_peers(vec![known.clone()]);
        let revocations = Revocations {
            refuses: true,
            ..Revocations::default()
        };
        let (fresh, last) = (
            entry(known.id, 0),
            entry(known.id, MAX_DELIVERY_ATTEMPTS - 1),
        );
        revocations
            .due
            .lock()
            .unwrap()
            .extend([fresh.clone(), last.clone()]);

        assert_eq!(deliver_all(&store, &revocations).await, 0);
        let settled = revocations.settled.lock().unwrap();
        assert!(
            matches!(settled[0], Settled::Retried(id, _) if id == fresh.id)
        );
        assert_eq!(settled[1], Settled::GaveUp(last.id));
    }
}
