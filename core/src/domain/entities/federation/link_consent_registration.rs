use crate::domain::dtos::federation::{LinkConsent, NewLinkConsent};

use async_trait::async_trait;
use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

/// Pending consents on the issuer (FED-46)
///
/// Transitions are conditional updates: a consent gets one login and is
/// confirmed at most once.
///
#[async_trait]
pub trait LinkConsentRegistration: Interface + Send + Sync {
    /// `NotCreated` when the peer's request already has a consent, so a
    /// redirect state is accepted once
    async fn create(
        &self,
        consent: NewLinkConsent,
    ) -> Result<CreateResponseKind<LinkConsent>, MappedErrors>;

    /// An unexpired, unconfirmed consent
    async fn get_open(
        &self,
        consent_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors>;

    /// Record who logged in: `Found` only the first time, on an open consent
    async fn attach_user(
        &self,
        consent_id: Uuid,
        local_user_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors>;

    /// Confirm: `Found` only for an open consent with a user attached
    async fn consume(
        &self,
        consent_id: Uuid,
    ) -> Result<FetchResponseKind<LinkConsent, Uuid>, MappedErrors>;

    /// Delete the consents past `expires_at`; returns how many were deleted
    async fn purge_expired(&self) -> Result<u64, MappedErrors>;
}
