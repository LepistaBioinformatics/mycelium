use crate::domain::dtos::federation::InstanceSelfDescription;

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

/// Outbound calls to a peer instance
///
/// Every failure (connection, timeout, non-2xx, oversized or malformed body)
/// is an error: for the circuit breaker they all count the same. Signing is
/// the caller's job; this port only carries the JWS.
///
#[async_trait]
pub trait PeerClient: Interface + Send + Sync {
    /// `GET <base_url>/.well-known/mycelium-instance`, unauthenticated
    async fn fetch_self_description(
        &self,
        base_url: &str,
    ) -> Result<InstanceSelfDescription, MappedErrors>;

    /// `POST <base_url><path>` carrying `signature` in the peer request
    /// signature header; returns the response body
    async fn post_signed(
        &self,
        base_url: &str,
        path: &str,
        body: Vec<u8>,
        signature: &str,
    ) -> Result<Vec<u8>, MappedErrors>;
}
