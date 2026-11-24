use super::{super::types::JsonRpcError, federation::to_result};
use crate::models::active_backend_modules::SqlAppModule;

use actix_web::web;
use myc_core::{
    domain::dtos::written_by::WrittenBy,
    use_cases::federation::{revoke_link, LinkRevoker},
};
use shaku::HasComponent;
use uuid::Uuid;

/// Revoke a link and queue a tombstone for the peer; the federation
/// dispatcher pushes it within a second (FED-47)
pub(super) async fn revoke_identity_link(
    app_module: &web::Data<SqlAppModule>,
    link_id: Uuid,
    revoker: LinkRevoker,
    revoked_by: WrittenBy,
) -> Result<serde_json::Value, JsonRpcError> {
    to_result(
        revoke_link(
            link_id,
            revoker,
            Some(revoked_by),
            Box::new(app_module.resolve_ref()),
            Box::new(app_module.resolve_ref()),
            Box::new(app_module.resolve_ref()),
        )
        .await,
    )
}
