use super::SignedPeerRequest;
use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{web, HttpResponse};
use myc_core::{
    domain::dtos::federation::LinkConfirmation,
    use_cases::federation::confirm_link_on_consumer,
};
use myc_http_tools::settings::FEDERATION_LINK_CONFIRM_PATH;
use serde::de::DeserializeOwned;
use shaku::HasComponent;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config.route(
        FEDERATION_LINK_CONFIRM_PATH,
        web::post().to(accept_link_confirmation),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// The issuer says a user confirmed a link this instance requested (FED-12)
#[tracing::instrument(name = "accept_link_confirmation", skip_all)]
pub(crate) async fn accept_link_confirmation(
    request: SignedPeerRequest,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Some(confirmation) = body::<LinkConfirmation>(&request) else {
        return HttpResponse::BadRequest().finish();
    };

    match confirm_link_on_consumer(
        &request.peer,
        confirmation,
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    {
        Ok(acknowledgement) => HttpResponse::Ok().json(acknowledgement),
        Err(err) => {
            tracing::warn!(%err, peer = %request.peer.instance_id, "link confirmation refused");
            HttpResponse::Conflict().finish()
        }
    }
}

fn body<T: DeserializeOwned>(request: &SignedPeerRequest) -> Option<T> {
    serde_json::from_slice(&request.body).ok()
}
