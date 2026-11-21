use crate::domain::dtos::{
    federation::{FederationPeer, NewFederationPeer, NewPeerKey, PeerStatus},
    written_by::WrittenBy,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{
        CreateResponseKind, DeletionResponseKind, UpdatingResponseKind,
    },
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

/// Changes to declared peers
///
/// Only local callers (the CLI, the dispatcher, use cases) reach this port;
/// no remote-callable API declares a peer (FED-07).
///
#[async_trait]
pub trait FederationPeerWrite: Interface + Send + Sync {
    /// `NotCreated` when a peer with the same `instance_id` already exists
    async fn declare(
        &self,
        peer: NewFederationPeer,
        created_by: Option<WrittenBy>,
    ) -> Result<CreateResponseKind<FederationPeer>, MappedErrors>;

    async fn update_status(
        &self,
        peer_id: Uuid,
        status: PeerStatus,
    ) -> Result<UpdatingResponseKind<FederationPeer>, MappedErrors>;

    /// Replace every stored key of the peer with `keys`
    async fn record_keys(
        &self,
        peer_id: Uuid,
        keys: Vec<NewPeerKey>,
    ) -> Result<(), MappedErrors>;

    async fn set_negotiated_kinds(
        &self,
        peer_id: Uuid,
        kinds: Vec<String>,
    ) -> Result<(), MappedErrors>;

    /// Reset the failure count and stamp `last_contact_at`
    async fn record_success(&self, peer_id: Uuid) -> Result<(), MappedErrors>;

    /// Count one more consecutive failure; returns the new count
    async fn record_failure(&self, peer_id: Uuid) -> Result<u32, MappedErrors>;

    /// Remove the peer with its keys and nonces
    async fn remove(
        &self,
        peer_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}
