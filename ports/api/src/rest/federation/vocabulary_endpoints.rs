use super::SignedPeerRequest;
use crate::{
    federation::{issue_assertion, PreparedFederation},
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{web, HttpResponse};
use myc_core::{
    domain::dtos::federation::{
        ResolutionAnswer, RoleVocabularyAssertion, GUEST_ROLE_VOCABULARY_KIND,
    },
    use_cases::federation::answer_vocabulary_request,
};
use myc_http_tools::settings::FEDERATION_VOCABULARY_PATH;
use mycelium_base::utils::errors::MappedErrors;
use shaku::HasComponent;

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config.route(
        FEDERATION_VOCABULARY_PATH,
        web::post().to(answer_vocabulary),
    );
}

/// A peer asks for this instance's guest-role vocabulary (FED-40)
///
/// Answers a signed `guest-role-vocabulary.v1` assertion whose subject is
/// this instance, or `unknown` while the peer may not have it.
///
#[tracing::instrument(name = "answer_vocabulary", skip_all)]
pub(crate) async fn answer_vocabulary(
    request: SignedPeerRequest,
    prepared: web::Data<Option<PreparedFederation>>,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Some(prepared) = prepared.get_ref() else {
        return HttpResponse::NotFound().finish();
    };

    let answer = answer_vocabulary_request(
        &request.peer,
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    .and_then(|payload| sign(prepared, &request, payload));

    match answer {
        Ok(answer) => HttpResponse::Ok().json(answer),
        Err(err) => {
            tracing::error!(%err, peer = %request.peer.instance_id, "vocabulary failed");
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn sign(
    prepared: &PreparedFederation,
    request: &SignedPeerRequest,
    payload: Option<RoleVocabularyAssertion>,
) -> Result<ResolutionAnswer, MappedErrors> {
    let Some(payload) = payload else {
        return Ok(ResolutionAnswer::Unknown);
    };

    issue_assertion(
        prepared,
        &request.peer,
        prepared.instance_id.to_string(),
        GUEST_ROLE_VOCABULARY_KIND,
        payload,
    )
    .map(|jws| ResolutionAnswer::Assertion { jws })
}
