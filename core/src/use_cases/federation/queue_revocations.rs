use crate::domain::{
    dtos::federation::{
        IdentityLink, PendingRevocation, Tombstone, TombstoneReason,
    },
    entities::{IdentityLinkRead, RevocationOutbox},
};

use chrono::Local;
use mycelium_base::utils::errors::MappedErrors;

/// Queue a tombstone for every peer holding an active link of these subjects
/// (FED-22); the issuer's own link table is the list of peers to tell
///
/// Returns how many were queued. Delivery is the outbox's job.
///
#[tracing::instrument(name = "queue_revocations", skip_all)]
pub async fn queue_revocations(
    emails: &[String],
    reason: TombstoneReason,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    outbox: Box<&dyn RevocationOutbox>,
) -> Result<u64, MappedErrors> {
    if emails.is_empty() {
        return Ok(0);
    }

    let links = link_read_repo.list_active_by_emails(emails).await?;
    queue_for_links(&links, reason, *outbox).await
}

/// Queue a tombstone for each link the peer can name
pub(super) async fn queue_for_links(
    links: &[IdentityLink],
    reason: TombstoneReason,
    outbox: &dyn RevocationOutbox,
) -> Result<u64, MappedErrors> {
    let issued_at = Local::now();
    let revocations = links
        .iter()
        .filter_map(|link| {
            Some(PendingRevocation {
                peer_id: link.peer_id,
                tombstone: Tombstone {
                    subject_ref: link.peer_subject_ref.clone()?,
                    reason,
                    issued_at,
                },
            })
        })
        .collect::<Vec<_>>();

    if revocations.is_empty() {
        return Ok(0);
    }

    outbox.enqueue(revocations).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::federation::{LinkStatus, PeerStatus, SubjectRef},
        use_cases::federation::{
            link_test_doubles::{peer, LinkStore},
            revocation_test_doubles::Revocations,
        },
    };

    use uuid::Uuid;

    fn link(email: &str, status: LinkStatus, peer_id: Uuid) -> IdentityLink {
        IdentityLink {
            id: Uuid::new_v4(),
            local_user_id: Uuid::new_v4(),
            local_email: email.to_string(),
            peer_id,
            peer_subject_ref: Some(SubjectRef::mint()),
            local_subject_ref: SubjectRef::mint(),
            status,
            created: Local::now(),
            established_at: None,
            revoked_at: None,
            revoked_by: None,
        }
    }

    #[tokio::test]
    async fn only_active_links_of_the_subjects_are_told() {
        let peer_id = peer(PeerStatus::Active).id;
        let store = LinkStore::default();
        let told = link("user-01@example.com", LinkStatus::Active, peer_id);
        store.links.lock().unwrap().extend([
            told.clone(),
            link("user-01@example.com", LinkStatus::Revoked, peer_id),
            link("user-02@example.com", LinkStatus::Active, peer_id),
        ]);
        let outbox = Revocations::default();

        let queued = queue_revocations(
            &["user-01@example.com".to_string()],
            TombstoneReason::GrantsChanged,
            Box::new(&store),
            Box::new(&outbox),
        )
        .await
        .unwrap();

        assert_eq!(queued, 1);
        let queued = outbox.queued.lock().unwrap();
        assert_eq!(queued[0].peer_id, peer_id);
        assert_eq!(
            Some(queued[0].tombstone.subject_ref.clone()),
            told.peer_subject_ref
        );
    }
}
