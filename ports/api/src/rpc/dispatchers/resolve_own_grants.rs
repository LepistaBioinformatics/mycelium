use super::{
    super::{errors::mapped_errors_to_jsonrpc_error, types::JsonRpcError},
    federation::to_result,
};
use crate::{
    federation::{resolution_limits, resolution_ports, PreparedFederation},
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::{
        dtos::{
            profile::{Owner, Profile},
            written_by::WrittenBy,
        },
        entities::PeerGrantResolver,
    },
    use_cases::federation::resolve_own_federated_grants,
};
use mycelium_base::utils::errors::execution_err;

/// Resolve the caller's grants on the peers they linked, on demand; a local
/// denial wins (FED-19)
pub(super) async fn resolve_own_grants(
    app_module: &web::Data<SqlAppModule>,
    req: Option<&HttpRequest>,
    prepared: &PreparedFederation,
    profile: &Profile,
    owner: Owner,
) -> Result<serde_json::Value, JsonRpcError> {
    let resolver = req
        .and_then(|req| req.app_data::<web::Data<dyn PeerGrantResolver>>())
        .ok_or_else(|| {
            mapped_errors_to_jsonrpc_error(execution_err(
                "federation is not wired",
            ))
        })?;
    let author = WrittenBy::new_from_user_with_email(owner.id, &owner.email);

    to_result(
        resolve_own_federated_grants(
            profile,
            owner.id,
            &author,
            resolution_limits(prepared),
            &resolution_ports(app_module, resolver.get_ref()),
        )
        .await,
    )
}
