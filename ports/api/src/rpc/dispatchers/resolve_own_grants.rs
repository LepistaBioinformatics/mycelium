use super::{
    super::{errors::mapped_errors_to_jsonrpc_error, types::JsonRpcError},
    federation::to_result,
};
use crate::{
    federation::PreparedFederation,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::{
        dtos::{
            federation::{FederatedResolution, PeerOutcome},
            profile::Owner,
            written_by::WrittenBy,
        },
        entities::PeerGrantResolver,
    },
    use_cases::federation::{
        resolve_federated_grants, ResolutionGuardPorts, ResolutionLimits,
        ResolutionPeerPorts, ResolutionPorts, ResolutionStorePorts,
    },
};
use mycelium_base::utils::errors::execution_err;
use serde::Serialize;
use shaku::HasComponent;
use std::time::Duration;

/// The federated part of the caller's grants, per linked peer (FED-11)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnGrants {
    /// Some linked peer could not contribute (FED-13)
    partial: bool,
    peers: Vec<PeerOutcome>,
}

/// Resolve the caller's grants on the peers they linked, on demand
pub(super) async fn resolve_own_grants(
    app_module: &web::Data<SqlAppModule>,
    req: Option<&HttpRequest>,
    prepared: &PreparedFederation,
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

    let resolution = resolve_federated_grants(
        owner.id,
        &author,
        limits_of(prepared),
        &ports(app_module, resolver.get_ref()),
    )
    .await;

    to_result(resolution.map(own_grants))
}

fn ports<'a>(
    app_module: &'a SqlAppModule,
    resolver: &'a dyn PeerGrantResolver,
) -> ResolutionPorts<'a> {
    ResolutionPorts {
        links: app_module.resolve_ref(),
        peers: ResolutionPeerPorts {
            read: app_module.resolve_ref(),
            write: app_module.resolve_ref(),
            resolver,
        },
        store: ResolutionStorePorts {
            read: app_module.resolve_ref(),
            write: app_module.resolve_ref(),
        },
        guard: ResolutionGuardPorts {
            probe: app_module.resolve_ref(),
            claims: app_module.resolve_ref(),
        },
        audit: app_module.resolve_ref(),
    }
}

fn limits_of(prepared: &PreparedFederation) -> ResolutionLimits {
    ResolutionLimits {
        deadline: Duration::from_millis(prepared.config.resolve_deadline_ms),
        max_failures_before_open: prepared.config.max_failures_before_open,
        assertion_ttl: Duration::from_secs(prepared.config.assertion_ttl_secs),
    }
}

fn own_grants(resolution: FederatedResolution) -> OwnGrants {
    OwnGrants {
        partial: resolution.is_partial(),
        peers: resolution.0,
    }
}
