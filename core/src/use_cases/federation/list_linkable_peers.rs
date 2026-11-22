use crate::domain::{
    dtos::federation::{LinkablePeer, PeerStatus},
    entities::FederationPeerRead,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// The active peers a user may link an account with, by name only (FED-43)
///
/// A peer without a display name is shown by its base URL.
///
#[tracing::instrument(name = "list_linkable_peers", skip_all)]
pub async fn list_linkable_peers(
    peer_read_repo: Box<&dyn FederationPeerRead>,
) -> Result<Vec<LinkablePeer>, MappedErrors> {
    let FetchManyResponseKind::Found(peers) = peer_read_repo.list().await?
    else {
        return Ok(vec![]);
    };

    Ok(peers
        .into_iter()
        .filter(|peer| peer.status == PeerStatus::Active)
        .map(|peer| LinkablePeer {
            instance_id: peer.instance_id,
            display_name: peer.display_name.unwrap_or(peer.base_url),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::federation::link_test_doubles::{peer, LinkStore};

    #[tokio::test]
    async fn only_active_peers_are_listed_by_name() {
        let named = peer(PeerStatus::Active);
        let mut unnamed = peer(PeerStatus::Active);
        unnamed.display_name = None;
        let store = LinkStore::with_peers(vec![
            named.clone(),
            unnamed.clone(),
            peer(PeerStatus::Quarantined),
            peer(PeerStatus::Declared),
        ]);

        let listed = list_linkable_peers(Box::new(&store)).await.unwrap();

        assert_eq!(
            listed,
            vec![
                LinkablePeer {
                    instance_id: named.instance_id,
                    display_name: "Instance 02".to_string(),
                },
                LinkablePeer {
                    instance_id: unnamed.instance_id,
                    display_name: unnamed.base_url,
                },
            ]
        );
    }
}
