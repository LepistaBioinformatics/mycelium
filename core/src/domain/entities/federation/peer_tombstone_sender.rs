use crate::domain::dtos::federation::{FederationPeer, Tombstone};

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

/// Push a tombstone to a peer in a signed request (FED-22, D-9)
#[async_trait]
pub trait PeerTombstoneSender: Interface + Send + Sync {
    async fn push(
        &self,
        peer: &FederationPeer,
        tombstone: Tombstone,
    ) -> Result<(), MappedErrors>;
}
