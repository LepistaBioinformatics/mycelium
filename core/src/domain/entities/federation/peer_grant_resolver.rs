use crate::domain::dtos::federation::{
    FederationPeer, PeerGrantAnswer, SubjectRef,
};

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

/// Signed resolution calls to a peer (FED-11, FED-14)
///
/// The implementation signs the request (D-9) and verifies the assertion
/// against the peer's keys, this instance as audience and `subject_ref` as
/// subject. A failed call is an error.
///
#[async_trait]
pub trait PeerGrantResolver: Interface + Send + Sync {
    async fn resolve(
        &self,
        peer: &FederationPeer,
        subject_ref: SubjectRef,
    ) -> Result<PeerGrantAnswer, MappedErrors>;
}
