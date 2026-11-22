use crate::domain::dtos::federation::{
    FederationPeer, LinkAcknowledgement, LinkConfirmation, LinkRevocation,
};

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

/// Signed calls that keep a peer's copy of a link in step (FED-12, FED-47)
///
/// The implementation signs each call with this instance's federation key
/// (D-9) and knows the peer's endpoints; use cases only say what happened.
///
#[async_trait]
pub trait PeerLinkNotifier: Interface + Send + Sync {
    /// Tell the consumer the user confirmed the link on this issuer; any
    /// refusal or failure is an error
    async fn confirm_link(
        &self,
        peer: &FederationPeer,
        confirmation: LinkConfirmation,
    ) -> Result<LinkAcknowledgement, MappedErrors>;

    /// Tell the peer a link was revoked on this side
    async fn revoke_link(
        &self,
        peer: &FederationPeer,
        revocation: LinkRevocation,
    ) -> Result<(), MappedErrors>;
}
