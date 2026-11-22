use super::{list_role_catalog, read_role_catalog_mode};
use crate::domain::{
    dtos::federation::{
        FederationPeer, PeerStatus, RoleCatalogMode, RoleVocabularyAssertion,
        GUEST_ROLE_VOCABULARY_KIND,
    },
    entities::{GuestRoleFetching, InstanceSettingsFetching},
};

use mycelium_base::utils::errors::MappedErrors;

/// The guest-role vocabulary this issuer vouches for, to a peer that asks
/// (FED-40)
///
/// Only an active peer that negotiated `guest-role-vocabulary.v1`, and only
/// while the role catalog is not `off` (D-10); `None` otherwise. The caller
/// signs the payload as an assertion of that kind.
///
#[tracing::instrument(name = "answer_vocabulary_request", skip_all)]
pub async fn answer_vocabulary_request(
    peer: &FederationPeer,
    settings_repo: Box<&dyn InstanceSettingsFetching>,
    guest_role_repo: Box<&dyn GuestRoleFetching>,
) -> Result<Option<RoleVocabularyAssertion>, MappedErrors> {
    if !may_ask(peer) {
        return Ok(None);
    }
    if read_role_catalog_mode(settings_repo).await? == RoleCatalogMode::Off {
        return Ok(None);
    }

    let roles = list_role_catalog(guest_role_repo).await?;
    Ok(Some(RoleVocabularyAssertion { roles }))
}

fn may_ask(peer: &FederationPeer) -> bool {
    peer.status == PeerStatus::Active
        && peer
            .negotiated_kinds
            .iter()
            .any(|kind| kind == GUEST_ROLE_VOCABULARY_KIND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::federation::link_test_doubles::peer;

    #[test]
    fn only_an_active_peer_that_negotiated_the_kind_may_ask() {
        let mut asking = peer(PeerStatus::Active);
        assert!(!may_ask(&asking));

        asking.negotiated_kinds = vec![GUEST_ROLE_VOCABULARY_KIND.to_string()];
        assert!(may_ask(&asking));

        asking.status = PeerStatus::Quarantined;
        assert!(!may_ask(&asking));
    }
}
