use super::{
    super::{errors::mapped_errors_to_jsonrpc_error, types::JsonRpcError},
    federation::to_result,
};
use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::{dtos::written_by::WrittenBy, entities::PeerLinkNotifier},
    use_cases::federation::{revoke_link, LinkRevoker},
};
use shaku::HasComponent;
use uuid::Uuid;

/// Revoke a link and tell the peer with the notifier the federation
/// endpoints were wired with (FED-47)
pub(super) async fn revoke_identity_link(
    app_module: &web::Data<SqlAppModule>,
    req: Option<&HttpRequest>,
    link_id: Uuid,
    revoker: LinkRevoker,
    revoked_by: WrittenBy,
) -> Result<serde_json::Value, JsonRpcError> {
    let notifier = req
        .and_then(|req| req.app_data::<web::Data<dyn PeerLinkNotifier>>())
        .ok_or_else(|| {
            mapped_errors_to_jsonrpc_error(
                mycelium_base::utils::errors::execution_err(
                    "federation is not wired",
                ),
            )
        })?;

    to_result(
        revoke_link(
            link_id,
            revoker,
            Some(revoked_by),
            Box::new(app_module.resolve_ref()),
            Box::new(app_module.resolve_ref()),
            Box::new(app_module.resolve_ref()),
            Box::new(notifier.get_ref()),
        )
        .await,
    )
}
