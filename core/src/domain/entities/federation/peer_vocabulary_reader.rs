use crate::domain::dtos::federation::{FederationPeer, PeerVocabularyAnswer};

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

/// Ask a peer for its guest-role vocabulary (FED-40)
///
/// The implementation signs the request (D-9) and verifies the assertion
/// against the peer's keys, this instance as audience, the peer itself as
/// subject and `guest-role-vocabulary.v1` as kind. A failed call is an
/// error; another kind is `Unknown` (FED-41).
///
#[async_trait]
pub trait PeerVocabularyReader: Interface + Send + Sync {
    async fn read(
        &self,
        peer: &FederationPeer,
    ) -> Result<PeerVocabularyAnswer, MappedErrors>;
}
