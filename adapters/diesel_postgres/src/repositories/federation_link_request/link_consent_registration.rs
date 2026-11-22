use super::map_link_request_rows::{found, map_consent_row};
use crate::{
    models::{config::DbPoolProvider, federation_link::LinkConsentRow},
    schema::federation_link_consent as consent_model,
};

use async_trait::async_trait;
use chrono::Utc;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::{LinkConsent, NewLinkConsent},
    entities::LinkConsentRegistration,
};
use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{creation_err, updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = LinkConsentRegistration)]
pub struct LinkConsentRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl LinkConsentRegistrationSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn update_err(e: diesel::result::Error) -> MappedErrors {
    updating_err(format!("Failed to update link consent: {e}"))
}

/// Unexpired and never confirmed
macro_rules! open_consent {
    ($consent_id:expr) => {
        consent_model::table
            .find($consent_id)
            .filter(consent_model::consumed_at.is_null())
            .filter(consent_model::expires_at.gt(Utc::now()))
    };
}

#[async_trait]
impl LinkConsentRegistration for LinkConsentRegistrationSqlDbRepository {
    #[tracing::instrument(name = "link_consent_create", skip_all)]
    async fn create(
        &self,
        consent: NewLinkConsent,
    ) -> Result<CreateResponseKind<LinkConsent>, MappedErrors> {
        let row = diesel::insert_into(consent_model::table)
            .values((
                consent_model::peer_id.eq(consent.peer_id),
                consent_model::peer_request_id.eq(consent.peer_request_id),
                consent_model::peer_email.eq(&consent.peer_email),
                consent_model::return_url.eq(&consent.return_url),
                consent_model::expires_at
                    .eq(consent.expires_at.with_timezone(&Utc)),
            ))
            .on_conflict_do_nothing()
            .returning(LinkConsentRow::as_returning())
            .get_result::<LinkConsentRow>(&mut *self.connection()?)
            .optional()
            .map_err(|e| {
                creation_err(format!("Failed to create consent: {e}"))
            })?;

        let Some(row) = row else {
            return Ok(CreateResponseKind::NotCreated(
                refused(consent),
                "this link request was already accepted".to_string(),
            ));
        };

        Ok(CreateResponseKind::Created(map_consent_row(row)))
    }

    #[tracing::instrument(name = "link_consent_get_open", skip_all)]
    async fn get_open(
        &self,
        consent_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors> {
        let row = open_consent!(consent_id)
            .select(LinkConsentRow::as_select())
            .first::<LinkConsentRow>(&mut *self.connection()?)
            .optional()
            .map_err(update_err)?;

        found(row, consent_id, map_consent_row)
    }

    #[tracing::instrument(name = "link_consent_attach_user", skip_all)]
    async fn attach_user(
        &self,
        consent_id: Uuid,
        local_user_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors> {
        let row = diesel::update(
            open_consent!(consent_id)
                .filter(consent_model::local_user_id.is_null()),
        )
        .set(consent_model::local_user_id.eq(Some(local_user_id)))
        .returning(LinkConsentRow::as_returning())
        .get_result::<LinkConsentRow>(&mut *self.connection()?)
        .optional()
        .map_err(update_err)?;

        found(row, consent_id, map_consent_row)
    }

    #[tracing::instrument(name = "link_consent_consume", skip_all)]
    async fn consume(
        &self,
        consent_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors> {
        let row = diesel::update(
            open_consent!(consent_id)
                .filter(consent_model::local_user_id.is_not_null()),
        )
        .set(consent_model::consumed_at.eq(Some(Utc::now())))
        .returning(LinkConsentRow::as_returning())
        .get_result::<LinkConsentRow>(&mut *self.connection()?)
        .optional()
        .map_err(update_err)?;

        found(row, consent_id, map_consent_row)
    }

    #[tracing::instrument(name = "link_consent_purge_expired", skip_all)]
    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        diesel::delete(
            consent_model::table
                .filter(consent_model::expires_at.le(Utc::now())),
        )
        .execute(&mut *self.connection()?)
        .map(|deleted| deleted as u64)
        .map_err(update_err)
    }
}

/// The refused consent as asked, so `NotCreated` carries what was refused
fn refused(consent: NewLinkConsent) -> LinkConsent {
    LinkConsent {
        id: Uuid::nil(),
        peer_id: consent.peer_id,
        peer_request_id: consent.peer_request_id,
        peer_email: consent.peer_email,
        return_url: consent.return_url,
        local_user_id: None,
        expires_at: consent.expires_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::{
        federation_link::live_link_schema::{
            live_pool, provider, user_and_peer,
        },
        federation_link_request::LinkRequestRegistrationSqlDbRepository,
    };

    use chrono::{Duration, Local};
    use myc_core::domain::{
        dtos::federation::{LinkRequestSecret, NewLinkRequest},
        entities::LinkRequestRegistration,
    };

    fn new_consent(peer_id: Uuid, minutes: i64) -> NewLinkConsent {
        NewLinkConsent {
            peer_id,
            peer_request_id: Uuid::new_v4(),
            peer_email: "user@myc-02.example.com".to_string(),
            return_url: "https://myc-02.example.com/federation/link/done"
                .to_string(),
            expires_at: Local::now() + Duration::minutes(minutes),
        }
    }

    #[tokio::test]
    async fn a_request_is_redirected_once_then_consumed_once() {
        let Some(pool) = live_pool() else { return };
        let (user_id, peer_id) = user_and_peer(&pool);
        let repo = LinkRequestRegistrationSqlDbRepository {
            db_config: provider(pool),
        };
        let secret = LinkRequestSecret::mint();
        let request = repo
            .create(NewLinkRequest {
                local_user_id: user_id,
                peer_id,
                secret_hash: secret.hash(),
                expires_at: Local::now() + Duration::minutes(10),
            })
            .await
            .unwrap();

        let not_redirected = repo.consume(request.id, peer_id).await.unwrap();
        assert!(matches!(not_redirected, FetchResponseKind::NotFound(_)));

        let wrong = repo
            .redirect(request.id, LinkRequestSecret::mint().hash())
            .await
            .unwrap();
        assert!(matches!(wrong, FetchResponseKind::NotFound(_)));

        let open = repo.get_open(request.id, secret.hash()).await.unwrap();
        assert!(matches!(open, FetchResponseKind::Found(_)));

        let first = repo.redirect(request.id, secret.hash()).await.unwrap();
        assert!(matches!(first, FetchResponseKind::Found(_)));
        let second = repo.redirect(request.id, secret.hash()).await.unwrap();
        assert!(matches!(second, FetchResponseKind::NotFound(_)));
        let spent = repo.get_open(request.id, secret.hash()).await.unwrap();
        assert!(matches!(spent, FetchResponseKind::NotFound(_)));

        let other_peer =
            repo.consume(request.id, Uuid::new_v4()).await.unwrap();
        assert!(matches!(other_peer, FetchResponseKind::NotFound(_)));
        let consumed = repo.consume(request.id, peer_id).await.unwrap();
        assert!(matches!(consumed, FetchResponseKind::Found(_)));
        let again = repo.consume(request.id, peer_id).await.unwrap();
        assert!(matches!(again, FetchResponseKind::NotFound(_)));
    }

    #[tokio::test]
    async fn a_consent_takes_one_login_and_one_confirmation() {
        let Some(pool) = live_pool() else { return };
        let (user_id, peer_id) = user_and_peer(&pool);
        let repo = LinkConsentRegistrationSqlDbRepository {
            db_config: provider(pool),
        };
        let asked = new_consent(peer_id, 10);

        let CreateResponseKind::Created(consent) =
            repo.create(asked.clone()).await.unwrap()
        else {
            panic!("the consent must be created");
        };
        let replayed = repo.create(asked).await.unwrap();
        assert!(matches!(replayed, CreateResponseKind::NotCreated(..)));

        let unattached = repo.consume(consent.id).await.unwrap();
        assert!(matches!(unattached, FetchResponseKind::NotFound(_)));

        let attached = repo.attach_user(consent.id, user_id).await.unwrap();
        assert!(matches!(attached, FetchResponseKind::Found(_)));
        let twice = repo.attach_user(consent.id, user_id).await.unwrap();
        assert!(matches!(twice, FetchResponseKind::NotFound(_)));

        let confirmed = repo.consume(consent.id).await.unwrap();
        assert!(matches!(confirmed, FetchResponseKind::Found(_)));
        let closed = repo.get_open(consent.id).await.unwrap();
        assert!(matches!(closed, FetchResponseKind::NotFound(_)));
    }

    #[tokio::test]
    async fn purge_removes_only_expired_consents() {
        let Some(pool) = live_pool() else { return };
        let (_, peer_id) = user_and_peer(&pool);
        let repo = LinkConsentRegistrationSqlDbRepository {
            db_config: provider(pool),
        };

        let CreateResponseKind::Created(live) =
            repo.create(new_consent(peer_id, 10)).await.unwrap()
        else {
            panic!("the live consent must be created");
        };
        let CreateResponseKind::Created(expired) =
            repo.create(new_consent(peer_id, -1)).await.unwrap()
        else {
            panic!("the expired consent must be created");
        };

        assert!(repo.purge_expired().await.unwrap() >= 1);
        assert!(matches!(
            repo.get_open(live.id).await.unwrap(),
            FetchResponseKind::Found(_)
        ));
        assert!(matches!(
            repo.get_open(expired.id).await.unwrap(),
            FetchResponseKind::NotFound(_)
        ));
    }
}
