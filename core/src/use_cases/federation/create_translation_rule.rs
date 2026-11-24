use super::{
    audit_translation_change::audit_translation_change,
    ensure_federation_operator, TranslationPorts,
};
use crate::domain::dtos::{
    federation::{GrantTranslationRule, NewGrantTranslationRule},
    profile::Profile,
    resource_audit_log::ResourceAuditEventKind,
    written_by::WrittenBy,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use serde_json::json;

/// Create a grant translation rule (FED-48, FED-49)
///
/// Staff only. The peer must be declared, and the target a pre-existing
/// guest role on a pre-existing tenant account that is not a system account:
/// translation never creates presence or privilege (I-3).
///
#[tracing::instrument(name = "create_translation_rule", skip_all)]
pub async fn create_translation_rule(
    profile: &Profile,
    rule: NewGrantTranslationRule,
    ports: TranslationPorts<'_>,
) -> Result<GrantTranslationRule, MappedErrors> {
    ensure_federation_operator(profile)?;
    check_targets(&rule, ports).await?;

    let author = WrittenBy::new_from_account(profile.acc_id);
    let created = ports.write.create(rule, Some(author.clone())).await?;
    let metadata = json!({ "issuer": created.issuer, "local": created.local });
    audit_translation_change(
        created.id,
        ResourceAuditEventKind::Created,
        metadata,
        &author,
        ports,
    )
    .await;

    Ok(created)
}

async fn check_targets(
    rule: &NewGrantTranslationRule,
    ports: TranslationPorts<'_>,
) -> Result<(), MappedErrors> {
    if !matches!(
        ports.peers.get(rule.peer_id).await?,
        FetchResponseKind::Found(_)
    ) {
        return refused("unknown peer");
    }

    let local = &rule.local;
    if ports
        .read
        .target(local.account_id, local.guest_role_id)
        .await?
        .is_none()
    {
        return refused(
            "the local account and guest role must exist, and the account \
             must belong to a tenant and not be a system account",
        );
    }

    Ok(())
}

fn refused<T>(reason: &str) -> Result<T, MappedErrors> {
    use_case_err(reason).with_exp_true().as_error()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            dtos::{
                federation::{
                    ActiveTranslation, IssuerGrantMatch, LocalGrant,
                    PeerStatus, TranslationTarget,
                },
                guest_role::Permission,
            },
            entities::{GrantTranslationRead, GrantTranslationWrite},
        },
        use_cases::federation::{
            link_test_doubles::{peer, LinkStore},
            resolution_test_doubles::Mesh,
        },
    };

    use async_trait::async_trait;
    use mycelium_base::entities::DeletionResponseKind;
    use uuid::Uuid;

    /// Targets exist unless `missing`
    #[derive(Default)]
    struct Rules {
        missing: bool,
    }

    #[async_trait]
    impl GrantTranslationRead for Rules {
        async fn list(
            &self,
            _: Option<Uuid>,
        ) -> Result<Vec<GrantTranslationRule>, MappedErrors> {
            Ok(vec![])
        }

        async fn list_active_for_peers(
            &self,
            _: &[Uuid],
        ) -> Result<Vec<ActiveTranslation>, MappedErrors> {
            Ok(vec![])
        }

        async fn target(
            &self,
            account_id: Uuid,
            role_id: Uuid,
        ) -> Result<Option<TranslationTarget>, MappedErrors> {
            Ok((!self.missing).then(|| TranslationTarget {
                account_id,
                account_name: "local-account".to_string(),
                tenant_id: Uuid::new_v4(),
                role_id,
                role_slug: "reviewer".to_string(),
            }))
        }
    }

    #[async_trait]
    impl GrantTranslationWrite for Rules {
        async fn create(
            &self,
            rule: NewGrantTranslationRule,
            _: Option<WrittenBy>,
        ) -> Result<GrantTranslationRule, MappedErrors> {
            Ok(GrantTranslationRule {
                id: Uuid::new_v4(),
                peer_id: rule.peer_id,
                issuer: rule.issuer,
                local: rule.local,
                enabled: true,
            })
        }

        async fn set_enabled(
            &self,
            _: Uuid,
            _: bool,
            _: Option<WrittenBy>,
        ) -> Result<Option<GrantTranslationRule>, MappedErrors> {
            Ok(None)
        }

        async fn delete(
            &self,
            rule_id: Uuid,
        ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
            Ok(DeletionResponseKind::NotDeleted(rule_id, String::new()))
        }
    }

    fn new_rule(peer_id: Uuid) -> NewGrantTranslationRule {
        NewGrantTranslationRule {
            peer_id,
            issuer: IssuerGrantMatch {
                tenant_id: Uuid::new_v4(),
                role: "editor".to_string(),
                min_perm: Permission::Read,
            },
            local: LocalGrant {
                account_id: Uuid::new_v4(),
                guest_role_id: Uuid::new_v4(),
                perm: Permission::Read,
            },
        }
    }

    fn staff() -> Profile {
        let mut profile = Profile::default();
        profile.is_staff = true;
        profile
    }

    async fn create(
        profile: &Profile,
        rules: &Rules,
        peers: &LinkStore,
        mesh: &Mesh,
    ) -> Result<GrantTranslationRule, MappedErrors> {
        let peer_id = peers.peers[0].id;
        let ports = TranslationPorts {
            read: rules,
            write: rules,
            peers,
            audit: mesh,
        };

        create_translation_rule(profile, new_rule(peer_id), ports).await
    }

    #[tokio::test]
    async fn staff_creates_an_audited_rule_on_an_existing_target() {
        let peers = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let mesh = Mesh::default();

        let rule = create(&staff(), &Rules::default(), &peers, &mesh)
            .await
            .unwrap();

        assert!(rule.enabled);
        assert_eq!(mesh.audited.lock().unwrap()[0].resource_id, rule.id);
    }

    #[tokio::test]
    async fn non_staff_and_missing_or_privileged_targets_are_refused() {
        let peers = LinkStore::with_peers(vec![peer(PeerStatus::Active)]);
        let mesh = Mesh::default();
        let missing = Rules { missing: true };

        assert!(
            create(&Profile::default(), &Rules::default(), &peers, &mesh)
                .await
                .is_err()
        );
        assert!(create(&staff(), &missing, &peers, &mesh).await.is_err());
        assert!(mesh.audited.lock().unwrap().is_empty());
    }
}
