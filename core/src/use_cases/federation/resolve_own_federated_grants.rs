use super::{
    locally_denied, resolve_federated_grants, ResolutionLimits, ResolutionPorts,
};
use crate::domain::dtos::{
    federation::ResolvedFederatedGrants, profile::Profile,
    written_by::WrittenBy,
};

use mycelium_base::utils::errors::MappedErrors;
use uuid::Uuid;

/// The federated grants of the user behind `profile`, unless a local denial
/// wins, in which case no peer is asked (FED-19)
#[tracing::instrument(name = "resolve_own_federated_grants", skip_all)]
pub async fn resolve_own_federated_grants(
    profile: &Profile,
    local_user_id: Uuid,
    author: &WrittenBy,
    limits: ResolutionLimits,
    ports: &ResolutionPorts<'_>,
) -> Result<ResolvedFederatedGrants, MappedErrors> {
    if let Some(reason) = locally_denied(profile) {
        return Ok(ResolvedFederatedGrants::denied(reason));
    }

    resolve_federated_grants(local_user_id, author, limits, ports)
        .await
        .map(ResolvedFederatedGrants::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::federation::{
        link_test_doubles::LinkStore, resolution_test_doubles::Mesh,
        ResolutionGuardPorts, ResolutionPeerPorts, ResolutionStorePorts,
    };

    use std::time::Duration;

    #[tokio::test]
    async fn a_local_denial_wins_and_no_peer_is_asked() {
        let (links, mesh) = (LinkStore::default(), Mesh::default());
        let ports = ResolutionPorts {
            links: &links,
            peers: ResolutionPeerPorts {
                read: &links,
                write: &mesh,
                resolver: &mesh,
            },
            store: ResolutionStorePorts {
                read: &mesh,
                write: &mesh,
            },
            guard: ResolutionGuardPorts {
                probe: &mesh,
                claims: &mesh,
            },
            audit: &mesh,
        };
        let limits = ResolutionLimits {
            deadline: Duration::from_millis(50),
            max_failures_before_open: 5,
            assertion_ttl: Duration::from_secs(60),
            assertion_refresh: Duration::from_secs(30),
        };
        let mut profile = Profile::default();
        profile.account_is_active = false;

        let resolved = resolve_own_federated_grants(
            &profile,
            Uuid::new_v4(),
            &WrittenBy::default(),
            limits,
            &ports,
        )
        .await
        .unwrap();

        assert!(resolved.denied.unwrap().contains("inactive"));
        assert!(resolved.peers.is_empty());
        assert!(mesh.asked.lock().unwrap().is_empty());
    }
}
