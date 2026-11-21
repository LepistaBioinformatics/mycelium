use crate::domain::dtos::federation::{FederationPeer, PeerKey};

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait FederationPeerRead: Interface + Send + Sync {
    async fn get(
        &self,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors>;

    /// The peer declared with this `instance_id`, the `iss` of its requests
    async fn get_by_instance_id(
        &self,
        instance_id: Uuid,
    ) -> Result<FetchResponseKind<FederationPeer, Uuid>, MappedErrors>;

    async fn list(
        &self,
    ) -> Result<FetchManyResponseKind<FederationPeer>, MappedErrors>;

    /// The stored public keys of a peer, empty before its first handshake
    async fn list_keys(
        &self,
        peer_id: Uuid,
    ) -> Result<Vec<PeerKey>, MappedErrors>;
}
