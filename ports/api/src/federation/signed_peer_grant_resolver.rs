use super::{AssertionVerifier, SignedPeerPoster};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::federation::{
        FederationPeer, PeerGrantAnswer, ResolutionAnswer, ResolutionRequest,
        SubjectRef, VerifiedAssertion,
    },
    entities::PeerGrantResolver,
};
use myc_http_tools::{
    federation::AssertionClaims, settings::FEDERATION_RESOLVE_PATH,
};
use mycelium_base::utils::errors::MappedErrors;

/// `PeerGrantResolver` over signed peer requests (D-9, FED-14)
pub(crate) struct SignedPeerGrantResolver {
    pub(crate) poster: SignedPeerPoster,
    pub(crate) verifier: AssertionVerifier,
}

#[async_trait]
impl PeerGrantResolver for SignedPeerGrantResolver {
    #[tracing::instrument(name = "peer_resolve_grants", skip_all)]
    async fn resolve(
        &self,
        peer: &FederationPeer,
        subject_ref: SubjectRef,
    ) -> Result<PeerGrantAnswer, MappedErrors> {
        let request = ResolutionRequest {
            subject_ref: subject_ref.clone(),
        };
        let body = self
            .poster
            .post(peer, FEDERATION_RESOLVE_PATH, &request)
            .await?;

        let Ok(answer) = serde_json::from_slice::<ResolutionAnswer>(&body)
        else {
            return Ok(PeerGrantAnswer::Invalid(
                "malformed answer".to_string(),
            ));
        };
        let ResolutionAnswer::Assertion { jws } = answer else {
            return Ok(PeerGrantAnswer::Unknown);
        };

        let verified = self
            .verifier
            .verify(peer, &jws, subject_ref.as_str())
            .await?;

        Ok(match verified {
            Ok(claims) => PeerGrantAnswer::Verified(verified_of(jws, claims)),
            Err(rejection) => PeerGrantAnswer::Invalid(rejection.to_string()),
        })
    }
}

fn verified_of(jws: String, claims: AssertionClaims) -> VerifiedAssertion {
    VerifiedAssertion {
        jws,
        payload: claims.payload,
        issued_at: local_time(claims.iat),
        expires_at: local_time(claims.exp),
    }
}

fn local_time(seconds: u64) -> DateTime<Local> {
    DateTime::from_timestamp(seconds as i64, 0)
        .unwrap_or_default()
        .with_timezone(&Local)
}
