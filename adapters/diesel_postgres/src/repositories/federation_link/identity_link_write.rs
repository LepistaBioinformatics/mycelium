use super::{
    activate_or_revoke_link::{activate_link, revoke_link},
    record_link::record_link,
};
use crate::models::config::DbPoolProvider;

use async_trait::async_trait;
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use myc_core::domain::{
    dtos::{
        federation::{IdentityLink, NewIdentityLink, SubjectRef},
        written_by::WrittenBy,
    },
    entities::IdentityLinkWrite,
};
use mycelium_base::{
    entities::{CreateResponseKind, UpdatingResponseKind},
    utils::errors::{updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = IdentityLinkWrite)]
pub struct IdentityLinkWriteSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl IdentityLinkWriteSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

#[async_trait]
impl IdentityLinkWrite for IdentityLinkWriteSqlDbRepository {
    #[tracing::instrument(name = "identity_link_record", skip_all)]
    async fn record(
        &self,
        link: NewIdentityLink,
    ) -> Result<CreateResponseKind<IdentityLink>, MappedErrors> {
        record_link(&mut *self.connection()?, link)
    }

    #[tracing::instrument(name = "identity_link_activate", skip_all)]
    async fn activate(
        &self,
        link_id: Uuid,
        peer_subject_ref: SubjectRef,
    ) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors> {
        activate_link(&mut *self.connection()?, link_id, peer_subject_ref)
    }

    #[tracing::instrument(name = "identity_link_revoke", skip_all)]
    async fn revoke(
        &self,
        link_id: Uuid,
        revoked_by: Option<WrittenBy>,
    ) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors> {
        revoke_link(&mut *self.connection()?, link_id, revoked_by)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::config::DbPool,
        repositories::federation_link::{
            live_link_schema::{live_pool, provider, user_and_peer},
            IdentityLinkReadSqlDbRepository,
        },
    };

    use myc_core::domain::{
        dtos::federation::{LinkStatus, NewIdentityLink, SubjectRef},
        entities::{IdentityLinkRead, IdentityLinkWrite},
    };
    use mycelium_base::entities::{
        CreateResponseKind, FetchResponseKind, UpdatingResponseKind,
    };
    use uuid::Uuid;

    fn repos(
        pool: DbPool,
    ) -> (
        IdentityLinkReadSqlDbRepository,
        IdentityLinkWriteSqlDbRepository,
    ) {
        let provider = provider(pool);

        (
            IdentityLinkReadSqlDbRepository {
                db_config: provider.clone(),
            },
            IdentityLinkWriteSqlDbRepository {
                db_config: provider,
            },
        )
    }

    fn pending(user_id: Uuid, peer_id: Uuid) -> NewIdentityLink {
        NewIdentityLink {
            local_user_id: user_id,
            local_email: "user@myc-01.example.com".to_string(),
            peer_id,
            peer_subject_ref: None,
            status: LinkStatus::Pending,
        }
    }

    #[tokio::test]
    async fn a_second_live_link_with_the_same_peer_is_refused() {
        let Some(pool) = live_pool() else { return };
        let (user_id, peer_id) = user_and_peer(&pool);
        let (read, write) = repos(pool);

        let CreateResponseKind::Created(first) =
            write.record(pending(user_id, peer_id)).await.unwrap()
        else {
            panic!("the first link must be recorded");
        };

        let second = write.record(pending(user_id, peer_id)).await.unwrap();
        let CreateResponseKind::NotCreated(existing, _) = second else {
            panic!("a second live link must be refused");
        };
        assert_eq!(existing.id, first.id);

        let FetchResponseKind::Found(live) =
            read.find_live(user_id, peer_id).await.unwrap()
        else {
            panic!("the live link must be found");
        };
        assert_eq!(live.status, LinkStatus::Pending);
        assert!(live.peer_subject_ref.is_none());
    }

    #[tokio::test]
    async fn activation_happens_once_and_revocation_frees_the_pair() {
        let Some(pool) = live_pool() else { return };
        let (user_id, peer_id) = user_and_peer(&pool);
        let (read, write) = repos(pool);

        let CreateResponseKind::Created(link) =
            write.record(pending(user_id, peer_id)).await.unwrap()
        else {
            panic!("the link must be recorded");
        };

        let peer_ref = SubjectRef::mint();
        let UpdatingResponseKind::Updated(active) =
            write.activate(link.id, peer_ref.clone()).await.unwrap()
        else {
            panic!("a pending link must activate");
        };
        assert_eq!(active.status, LinkStatus::Active);
        assert_eq!(active.peer_subject_ref, Some(peer_ref));
        assert!(active.established_at.is_some());

        let again = write.activate(link.id, SubjectRef::mint()).await.unwrap();
        assert!(matches!(again, UpdatingResponseKind::NotUpdated(..)));

        let UpdatingResponseKind::Updated(revoked) =
            write.revoke(link.id, None).await.unwrap()
        else {
            panic!("a live link must revoke");
        };
        assert_eq!(revoked.status, LinkStatus::Revoked);
        assert!(matches!(
            write.revoke(link.id, None).await.unwrap(),
            UpdatingResponseKind::NotUpdated(..)
        ));

        let FetchResponseKind::Found(by_ref) = read
            .find_by_local_ref(peer_id, link.local_subject_ref.clone())
            .await
            .unwrap()
        else {
            panic!("the link must be found by its reference");
        };
        assert_eq!(by_ref.id, link.id);

        let relinked = write.record(pending(user_id, peer_id)).await.unwrap();
        assert!(matches!(relinked, CreateResponseKind::Created(_)));
        assert_eq!(read.list(Some(user_id)).await.unwrap().len(), 2);
    }
}
