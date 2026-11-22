use super::{describe_this_instance, PreparedFederation};

use chrono::{Local, Utc};
use myc_core::{
    domain::{
        dtos::federation::{FederationPeer, PeerKey},
        entities::{FederationPeerRead, FederationPeerWrite, PeerClient},
    },
    use_cases::federation::{handshake_peer, peer_keys_refresh_due},
};
use myc_http_tools::federation::{
    verify_assertion, AssertionClaims, ExpectedAssertion, PeerRequestRejection,
};
use mycelium_base::utils::errors::MappedErrors;
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};

/// Least time between two key refreshes an assertion can trigger for a peer
const KEY_REFRESH_COOL_DOWN: Duration = Duration::from_secs(30);

/// What verifying a peer's assertion needs
pub(crate) struct AssertionVerifier {
    pub(crate) prepared: PreparedFederation,
    pub(crate) peer_client: Arc<dyn PeerClient>,
    pub(crate) peer_read: Arc<dyn FederationPeerRead>,
    pub(crate) peer_write: Arc<dyn FederationPeerWrite>,
}

impl AssertionVerifier {
    /// Verify an assertion of `kind` with the stored keys; an unknown `kid`
    /// refreshes them once, at most every `KEY_REFRESH_COOL_DOWN` (FED-14)
    pub(crate) async fn verify<P: DeserializeOwned>(
        &self,
        peer: &FederationPeer,
        jws: &str,
        subject: &str,
        kind: &str,
    ) -> Result<Result<AssertionClaims<P>, PeerRequestRejection>, MappedErrors>
    {
        let expected = self.expected(peer, subject, kind);
        let keys = self.peer_read.list_keys(peer.id).await?;

        match verify_assertion(jws, &keys, &expected) {
            Err(PeerRequestRejection::UnknownKid(_))
                if self.refresh_due(&keys, peer) =>
            {
                self.refresh(peer).await?;
                let keys = self.peer_read.list_keys(peer.id).await?;
                Ok(verify_assertion(jws, &keys, &expected))
            }
            outcome => Ok(outcome),
        }
    }

    fn expected(
        &self,
        peer: &FederationPeer,
        subject: &str,
        kind: &str,
    ) -> ExpectedAssertion {
        ExpectedAssertion {
            issuer: peer.instance_id,
            audience: self.prepared.instance_id,
            subject: subject.to_string(),
            kind: kind.to_string(),
            now: Utc::now().timestamp().max(0) as u64,
            tolerance_secs: self.prepared.config.request_tolerance_secs,
        }
    }

    fn refresh_due(&self, keys: &[PeerKey], peer: &FederationPeer) -> bool {
        peer_keys_refresh_due(
            keys,
            peer.updated,
            Local::now(),
            KEY_REFRESH_COOL_DOWN,
        )
    }

    async fn refresh(&self, peer: &FederationPeer) -> Result<(), MappedErrors> {
        handshake_peer(
            peer,
            &describe_this_instance(&self.prepared, false).assertion_kinds,
            self.prepared.config.max_failures_before_open,
            Box::new(&*self.peer_client),
            Box::new(&*self.peer_write),
        )
        .await
        .map(|_| ())
    }
}
