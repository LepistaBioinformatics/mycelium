use super::{AssertionVerifier, SignedPeerPoster};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::federation::{
        FederationPeer, PeerVocabularyAnswer, ResolutionAnswer,
        RoleVocabularyAssertion, GUEST_ROLE_VOCABULARY_KIND,
    },
    entities::PeerVocabularyReader,
};
use myc_http_tools::{
    federation::{AssertionClaims, PeerRequestRejection},
    settings::FEDERATION_VOCABULARY_PATH,
};
use mycelium_base::utils::errors::MappedErrors;

/// `PeerVocabularyReader` over signed peer requests (D-9, FED-40)
pub(crate) struct SignedPeerVocabularyReader {
    pub(crate) poster: SignedPeerPoster,
    pub(crate) verifier: AssertionVerifier,
}

#[async_trait]
impl PeerVocabularyReader for SignedPeerVocabularyReader {
    #[tracing::instrument(name = "peer_read_vocabulary", skip_all)]
    async fn read(
        &self,
        peer: &FederationPeer,
    ) -> Result<PeerVocabularyAnswer, MappedErrors> {
        let body = self
            .poster
            .post(peer, FEDERATION_VOCABULARY_PATH, &serde_json::json!({}))
            .await?;

        let Ok(ResolutionAnswer::Assertion { jws }) =
            serde_json::from_slice(&body)
        else {
            return Ok(PeerVocabularyAnswer::Unknown);
        };

        let subject = peer.instance_id.to_string();
        let verified = self
            .verifier
            .verify(peer, &jws, &subject, GUEST_ROLE_VOCABULARY_KIND)
            .await?;

        Ok(answer_of(peer, verified))
    }
}

fn answer_of(
    peer: &FederationPeer,
    verified: Result<
        AssertionClaims<RoleVocabularyAssertion>,
        PeerRequestRejection,
    >,
) -> PeerVocabularyAnswer {
    match verified {
        Ok(claims) => PeerVocabularyAnswer::Verified {
            roles: claims.payload.roles,
        },
        Err(PeerRequestRejection::UnknownKind(kind)) => {
            tracing::warn!(%kind, peer = %peer.instance_id, "assertion of another kind ignored");
            PeerVocabularyAnswer::Unknown
        }
        Err(rejection) => PeerVocabularyAnswer::Rejected {
            reason: rejection.to_string(),
        },
    }
}
