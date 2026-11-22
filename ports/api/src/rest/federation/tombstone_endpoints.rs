use super::SignedPeerRequest;
use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{web, HttpResponse};
use myc_core::{
    domain::dtos::federation::Tombstone, use_cases::federation::apply_tombstone,
};
use myc_http_tools::settings::FEDERATION_TOMBSTONE_PATH;
use shaku::HasComponent;

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config.route(FEDERATION_TOMBSTONE_PATH, web::post().to(accept_tombstone));
}

/// A peer revoked grants or a link on its side (FED-22, FED-23). Always 204
/// for a well-formed call: the answer never tells which references exist.
#[tracing::instrument(name = "accept_tombstone", skip_all)]
pub(crate) async fn accept_tombstone(
    request: SignedPeerRequest,
    sql_module: web::Data<SqlAppModule>,
) -> HttpResponse {
    let Ok(tombstone) = serde_json::from_slice::<Tombstone>(&request.body)
    else {
        return HttpResponse::BadRequest().finish();
    };

    match apply_tombstone(
        &request.peer,
        tombstone,
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
        Box::new(sql_module.resolve_ref()),
    )
    .await
    {
        Ok(()) => HttpResponse::NoContent().finish(),
        Err(err) => {
            tracing::error!(%err, peer = %request.peer.instance_id, "tombstone failed");
            HttpResponse::InternalServerError().finish()
        }
    }
}
