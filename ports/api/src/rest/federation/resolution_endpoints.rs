use super::SignedPeerRequest;
use crate::{
    federation::{issue_profile_assertion, PreparedFederation},
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{web, HttpResponse};
use myc_core::{
    domain::dtos::federation::{
        FederationPeer, ProfileAssertion, ResolutionAnswer, ResolutionRequest,
    },
    use_cases::federation::answer_peer_resolution,
};
use myc_http_tools::settings::FEDERATION_RESOLVE_PATH;
use mycelium_base::utils::errors::MappedErrors;
use shaku::HasComponent;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config.route(FEDERATION_RESOLVE_PATH, web::post().to(answer_resolution));
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// A peer asks for the grants of a subject linked with it (FED-11, FED-12)
///
/// Answers a signed assertion, or `unknown` for any reference without an
/// active link with the caller.
///
#[tracing::instrument(name = "answer_resolution", skip_all)]
pub(crate) async fn answer_resolution(
    request: SignedPeerRequest,
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let (Some(prepared), Ok(asked)) = (
        prepared.get_ref(),
        serde_json::from_slice::<ResolutionRequest>(&request.body),
    ) else {
        return HttpResponse::BadRequest().finish();
    };

    let answer = answer_peer_resolution(
        &request.peer,
        asked.subject_ref.clone(),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    .and_then(|payload| sign(prepared, &request.peer, &asked, payload));

    match answer {
        Ok(answer) => HttpResponse::Ok().json(answer),
        Err(err) => {
            tracing::error!(%err, peer = %request.peer.instance_id, "resolution failed");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn sign(
    prepared: &PreparedFederation,
    peer: &FederationPeer,
    asked: &ResolutionRequest,
    payload: Option<ProfileAssertion>,
) -> Result<ResolutionAnswer, MappedErrors> {
    let Some(payload) = payload else {
        return Ok(ResolutionAnswer::Unknown);
    };

    issue_profile_assertion(prepared, peer, &asked.subject_ref, payload)
        .map(|jws| ResolutionAnswer::Assertion { jws })
}
