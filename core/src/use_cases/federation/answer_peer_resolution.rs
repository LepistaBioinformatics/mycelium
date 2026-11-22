use crate::domain::{
    dtos::{
        email::Email,
        federation::{
            FederatedGrants, FederationPeer, IdentityLink, LinkStatus,
            PeerStatus, ProfileAssertion, SubjectRef, PROFILE_ASSERTION_KIND,
        },
    },
    entities::{
        IdentityLinkRead, LicensedResourcesFetching, SubjectStandingFetching,
    },
};

use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};

/// What this issuer asserts about a subject a peer names (FED-12, FED-14)
///
/// `None` ("unknown") unless the reference names an active link with the
/// calling peer, and the peer is active and negotiated `profile.v1`. The
/// answer reveals nothing else (R-9). System-account grants are instance
/// privileges, like staff, and are never asserted (FED-28).
///
#[tracing::instrument(name = "answer_peer_resolution", skip_all)]
pub async fn answer_peer_resolution(
    peer: &FederationPeer,
    subject_ref: SubjectRef,
    link_read_repo: Box<&dyn IdentityLinkRead>,
    standing_repo: Box<&dyn SubjectStandingFetching>,
    licensed_resources_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<Option<ProfileAssertion>, MappedErrors> {
    if !may_ask(peer) {
        return Ok(None);
    }

    let Some(link) = active_link(peer, subject_ref, *link_read_repo).await?
    else {
        return Ok(None);
    };

    if denied(&link, *standing_repo).await? {
        return Ok(None);
    }

    let grants = grants_of(&link, *licensed_resources_repo).await?;
    Ok(Some(ProfileAssertion::from_grants(grants)))
}

fn may_ask(peer: &FederationPeer) -> bool {
    peer.status == PeerStatus::Active
        && peer
            .negotiated_kinds
            .iter()
            .any(|kind| kind == PROFILE_ASSERTION_KIND)
}

async fn active_link(
    peer: &FederationPeer,
    subject_ref: SubjectRef,
    link_read_repo: &dyn IdentityLinkRead,
) -> Result<Option<IdentityLink>, MappedErrors> {
    match link_read_repo
        .find_by_local_ref(peer.id, subject_ref)
        .await?
    {
        FetchResponseKind::Found(link) if link.status == LinkStatus::Active => {
            Ok(Some(link))
        }
        _ => Ok(None),
    }
}

/// The subject is denied on this instance, so nothing is vouched for: a
/// deactivation is not undone by the consumer's next refresh (FED-19)
async fn denied(
    link: &IdentityLink,
    standing_repo: &dyn SubjectStandingFetching,
) -> Result<bool, MappedErrors> {
    let standing = standing_repo.standing(link.local_user_id).await?;

    Ok(standing.is_none_or(|standing| standing.denial().is_some()))
}

async fn grants_of(
    link: &IdentityLink,
    licensed_resources_repo: &dyn LicensedResourcesFetching,
) -> Result<FederatedGrants, MappedErrors> {
    let email = Email::from_string(link.local_email.clone())?;
    let resources = licensed_resources_repo
        .list_licensed_resources(email, None, None, None, None)
        .await?;

    let FetchManyResponseKind::Found(resources) = resources else {
        return Ok(FederatedGrants::default());
    };

    Ok(FederatedGrants(
        resources
            .into_iter()
            .filter(|grant| !grant.sys_acc)
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::federation::SubjectStanding;
    use crate::{
        domain::{
            dtos::{guest_role::Permission, profile::LicensedResource},
            entities::{
                MockLicensedResourcesFetching, MockSubjectStandingFetching,
            },
        },
        use_cases::federation::link_test_doubles::{peer, LinkStore},
    };

    use chrono::Local;
    use uuid::Uuid;

    fn link(peer_id: Uuid, status: LinkStatus) -> IdentityLink {
        IdentityLink {
            id: Uuid::new_v4(),
            local_user_id: Uuid::new_v4(),
            local_email: "user-01@example.com".to_string(),
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

    fn grant(sys_acc: bool) -> LicensedResource {
        LicensedResource {
            acc_id: Uuid::new_v4(),
            sys_acc,
            tenant_id: Uuid::new_v4(),
            acc_name: "account-01".to_string(),
            role: "reader".to_string(),
            role_id: Uuid::new_v4(),
            perm: Permission::Read,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }
    }

    fn resources(
        grants: Vec<LicensedResource>,
    ) -> MockLicensedResourcesFetching {
        let mut repo = MockLicensedResourcesFetching::new();
        repo.expect_list_licensed_resources().returning(
            move |_, _, _, _, _| {
                Ok(FetchManyResponseKind::Found(grants.clone()))
            },
        );
        repo
    }

    fn asking_peer() -> FederationPeer {
        let mut asking = peer(PeerStatus::Active);
        asking.negotiated_kinds = vec![PROFILE_ASSERTION_KIND.to_string()];
        asking
    }

    fn standing(account_is_active: bool) -> MockSubjectStandingFetching {
        let mut repo = MockSubjectStandingFetching::new();
        repo.expect_standing().returning(move |_| {
            Ok(Some(SubjectStanding {
                owner_is_active: true,
                account_is_active,
                account_was_archived: false,
                account_was_deleted: false,
            }))
        });
        repo
    }

    async fn answer(
        asking: &FederationPeer,
        store: &LinkStore,
        subject_ref: SubjectRef,
    ) -> Option<ProfileAssertion> {
        answer_as(asking, store, subject_ref, standing(true)).await
    }

    async fn answer_as(
        asking: &FederationPeer,
        store: &LinkStore,
        subject_ref: SubjectRef,
        standing: MockSubjectStandingFetching,
    ) -> Option<ProfileAssertion> {
        let repo = resources(vec![grant(false), grant(true)]);
        answer_peer_resolution(
            asking,
            subject_ref,
            Box::new(store),
            Box::new(&standing),
            Box::new(&repo),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn an_active_link_with_the_caller_gets_its_non_system_grants() {
        let asking = asking_peer();
        let active = link(asking.id, LinkStatus::Active);
        let store = LinkStore::default();
        store.links.lock().unwrap().push(active.clone());

        let assertion = answer(&asking, &store, active.local_subject_ref)
            .await
            .unwrap();

        assert_eq!(assertion.grants.len(), 1);
        assert!(!assertion.grants.0[0].sys_acc);
        assert_eq!(assertion.issuer_namespace, assertion.grants.namespace());
    }

    #[tokio::test]
    async fn every_other_case_is_unknown() {
        let asking = asking_peer();
        let other_peer = link(Uuid::new_v4(), LinkStatus::Active);
        let pending = link(asking.id, LinkStatus::Pending);
        let store = LinkStore::default();
        store
            .links
            .lock()
            .unwrap()
            .extend([other_peer.clone(), pending.clone()]);
        let mut no_kind = asking.clone();
        no_kind.negotiated_kinds.clear();
        let mut quarantined = asking.clone();
        quarantined.status = PeerStatus::Quarantined;

        let refs = [
            other_peer.local_subject_ref,
            pending.local_subject_ref.clone(),
        ];
        for subject_ref in refs {
            assert!(answer(&asking, &store, subject_ref).await.is_none());
        }
        for caller in [no_kind, quarantined] {
            let subject_ref = pending.local_subject_ref.clone();
            assert!(answer(&caller, &store, subject_ref).await.is_none());
        }
        assert!(answer(&asking, &store, SubjectRef::mint()).await.is_none());
    }

    #[tokio::test]
    async fn a_subject_denied_on_the_issuer_is_unknown() {
        let asking = asking_peer();
        let active = link(asking.id, LinkStatus::Active);
        let store = LinkStore::default();
        store.links.lock().unwrap().push(active.clone());

        let subject_ref = active.local_subject_ref;
        let answer = answer_as(&asking, &store, subject_ref, standing(false));

        assert!(answer.await.is_none());
    }
}
