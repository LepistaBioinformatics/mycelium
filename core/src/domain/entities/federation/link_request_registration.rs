use crate::domain::dtos::federation::{LinkRequest, NewLinkRequest};

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

/// Single-use link initiations on the consumer (FED-42)
///
/// Both transitions are one conditional update: of two concurrent calls on
/// the same request, at most one gets `Found`.
///
#[async_trait]
pub trait LinkRequestRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        request: NewLinkRequest,
    ) -> Result<LinkRequest, MappedErrors>;

    /// An unexpired, never redirected request whose stored hash equals
    /// `secret_hash`, without spending it (a link scanner may open the URL)
    async fn get_open(
        &self,
        request_id: Uuid,
        secret_hash: String,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors>;

    /// Spend the initiation URL: `Found` only for an unexpired request,
    /// never redirected, whose stored hash equals `secret_hash`
    async fn redirect(
        &self,
        request_id: Uuid,
        secret_hash: String,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors>;

    /// Consume on the issuer's confirmation: `Found` only for an unexpired,
    /// redirected, unconsumed request made towards `peer_id`
    async fn consume(
        &self,
        request_id: Uuid,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<LinkRequest, Uuid>, MappedErrors>;

    /// Delete the requests past `expires_at`; returns how many were deleted
    async fn purge_expired(&self) -> Result<u64, MappedErrors>;
}
