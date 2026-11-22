use crate::domain::{
    dtos::federation::{
        ForeignIdKind, IssuerNamespace, NamespaceClaim, ProfileAssertion,
    },
    entities::{LocalNamespaceProbe, NamespaceClaimRegistration},
};

use mycelium_base::utils::errors::MappedErrors;
use uuid::Uuid;

/// Check every foreign id of an assertion against local ids and other
/// peers' claims, claiming the free ones for `peer_id` (I-1, FED-29, FED-54)
///
/// Returns the reason of the first violation, if any: an id that names a
/// local tenant or account, or that another peer claimed first. Local ids
/// always win; among peers, the first claim wins.
///
#[tracing::instrument(name = "guard_namespace_collisions", skip_all)]
pub async fn guard_namespace_collisions(
    peer_id: Uuid,
    assertion: &ProfileAssertion,
    probe_repo: Box<&dyn LocalNamespaceProbe>,
    claim_repo: Box<&dyn NamespaceClaimRegistration>,
) -> Result<Option<String>, MappedErrors> {
    let claimed = claimed_ids(assertion);

    let local = probe_repo.existing(&ids_of(&claimed)).await?;
    if !local.is_empty() {
        return Ok(Some(format!("ids exist on this instance: {local:?}")));
    }

    for (foreign_id, kind) in claimed {
        let claim = NamespaceClaim {
            foreign_id,
            peer_id,
            kind,
        };
        let owner = claim_repo.claim(claim).await?;
        if owner != peer_id {
            return Ok(Some(format!(
                "id {foreign_id} belongs to another peer"
            )));
        }
    }

    Ok(None)
}

/// The declared namespace and the ids the grants point at, each once
fn claimed_ids(assertion: &ProfileAssertion) -> Vec<(Uuid, ForeignIdKind)> {
    let mut claimed = kinds_of(&assertion.issuer_namespace);

    for id in kinds_of(&assertion.grants.namespace()) {
        if !claimed.contains(&id) {
            claimed.push(id);
        }
    }

    claimed
}

fn kinds_of(namespace: &IssuerNamespace) -> Vec<(Uuid, ForeignIdKind)> {
    let tenants = namespace
        .tenants
        .iter()
        .map(|id| (*id, ForeignIdKind::Tenant));
    let accounts = namespace
        .accounts
        .iter()
        .map(|id| (*id, ForeignIdKind::Account));

    tenants.chain(accounts).collect()
}

fn ids_of(claimed: &[(Uuid, ForeignIdKind)]) -> Vec<Uuid> {
    claimed.iter().map(|(id, _)| *id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        federation::FederatedGrants, guest_role::Permission,
        profile::LicensedResource,
    };

    use async_trait::async_trait;
    use std::{collections::HashMap, sync::Mutex};

    struct LocalIds(Vec<Uuid>);

    #[async_trait]
    impl LocalNamespaceProbe for LocalIds {
        async fn existing(
            &self,
            ids: &[Uuid],
        ) -> Result<Vec<Uuid>, MappedErrors> {
            Ok(ids
                .iter()
                .filter(|id| self.0.contains(id))
                .copied()
                .collect())
        }
    }

    #[derive(Default)]
    struct Claims(Mutex<HashMap<Uuid, Uuid>>);

    #[async_trait]
    impl NamespaceClaimRegistration for Claims {
        async fn claim(
            &self,
            claim: NamespaceClaim,
        ) -> Result<Uuid, MappedErrors> {
            let mut claims = self.0.lock().unwrap();
            Ok(*claims.entry(claim.foreign_id).or_insert(claim.peer_id))
        }
    }

    fn assertion(tenant_id: Uuid, acc_id: Uuid) -> ProfileAssertion {
        ProfileAssertion::from_grants(FederatedGrants(vec![LicensedResource {
            acc_id,
            sys_acc: false,
            tenant_id,
            acc_name: "account-01".to_string(),
            role: "reader".to_string(),
            role_id: Uuid::new_v4(),
            perm: Permission::Read,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }]))
    }

    async fn guard(
        peer_id: Uuid,
        assertion: &ProfileAssertion,
        local: &LocalIds,
        claims: &Claims,
    ) -> Option<String> {
        guard_namespace_collisions(
            peer_id,
            assertion,
            Box::new(local),
            Box::new(claims),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn free_ids_are_claimed_and_accepted_again_for_the_same_peer() {
        let (peer, tenant, account) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let claims = Claims::default();
        let local = LocalIds(vec![]);
        let asserted = assertion(tenant, account);

        assert_eq!(guard(peer, &asserted, &local, &claims).await, None);
        assert_eq!(guard(peer, &asserted, &local, &claims).await, None);
        assert_eq!(claims.0.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_local_id_or_another_peers_claim_is_a_violation() {
        let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
        let (tenant, account) = (Uuid::new_v4(), Uuid::new_v4());
        let claims = Claims::default();
        let asserted = assertion(tenant, account);

        let local = LocalIds(vec![account]);
        let collision = guard(first, &asserted, &local, &claims).await;
        assert!(collision.unwrap().contains("exist on this instance"));
        assert!(claims.0.lock().unwrap().is_empty());

        let local = LocalIds(vec![]);
        assert_eq!(guard(first, &asserted, &local, &claims).await, None);
        let squatting = guard(second, &asserted, &local, &claims).await;
        assert!(squatting.unwrap().contains("another peer"));
    }

    #[tokio::test]
    async fn an_undeclared_id_in_the_grants_is_checked_too() {
        let (peer, tenant, account) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut asserted = assertion(tenant, account);
        asserted.issuer_namespace = IssuerNamespace::default();

        let local = LocalIds(vec![tenant]);
        let collision =
            guard(peer, &asserted, &local, &Claims::default()).await;

        assert!(collision.is_some());
    }
}
