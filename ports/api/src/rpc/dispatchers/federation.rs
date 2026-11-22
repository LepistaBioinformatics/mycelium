use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    method_names,
    params::{FederationPeerParams, SetFederationEnabledParams},
    types::{self, JsonRpcError},
};
use crate::{
    dtos::MyceliumProfileData, federation::PreparedFederation,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::dtos::{profile::Profile, written_by::WrittenBy},
    use_cases::federation::{
        ensure_federation_operator, list_federation_peers, quarantine_peer,
        release_peer, set_federation_enabled,
    },
};
use serde::{de::DeserializeOwned, Serialize};
use shaku::HasComponent;

/// The staff-only `federation.*` scope (FED-33, FED-34, FED-35)
pub async fn dispatch_federation(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    req: Option<&HttpRequest>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    let profile = profile.to_profile();
    ensure_federation_operator(&profile)
        .map_err(mapped_errors_to_jsonrpc_error)?;

    match method {
        method_names::FEDERATION_PEERS_LIST => to_result(
            list_federation_peers(Box::new(app_module.resolve_ref())).await,
        ),
        method_names::FEDERATION_PEERS_QUARANTINE
        | method_names::FEDERATION_PEERS_RELEASE => {
            change_peer(app_module, method, parse(params)?).await
        }
        method_names::FEDERATION_SET_ENABLED => {
            let enabled = parse::<SetFederationEnabledParams>(params)?.enabled;
            switch_federation(app_module, req, &profile, enabled).await
        }
        _ => Err(method_not_found(method)),
    }
}

fn method_not_found(method: &str) -> JsonRpcError {
    JsonRpcError {
        code: types::codes::METHOD_NOT_FOUND,
        message: format!("Method not found: {}", method),
        data: None,
    }
}

async fn change_peer(
    app_module: &web::Data<SqlAppModule>,
    method: &str,
    params: FederationPeerParams,
) -> Result<serde_json::Value, JsonRpcError> {
    let read = Box::new(app_module.resolve_ref());
    let write = Box::new(app_module.resolve_ref());

    match method {
        method_names::FEDERATION_PEERS_QUARANTINE => {
            to_result(quarantine_peer(params.instance_id, read, write).await)
        }
        _ => to_result(release_peer(params.instance_id, read, write).await),
    }
}

async fn switch_federation(
    app_module: &web::Data<SqlAppModule>,
    req: Option<&HttpRequest>,
    profile: &Profile,
    enabled: bool,
) -> Result<serde_json::Value, JsonRpcError> {
    let prepared = req.and_then(|req| {
        req.app_data::<web::Data<Option<PreparedFederation>>>()
    });
    let config = prepared
        .and_then(|data| data.get_ref().as_ref())
        .map(|prepared| &prepared.config);

    to_result(
        set_federation_enabled(
            enabled,
            config,
            Some(WrittenBy::new_from_account(profile.acc_id)),
            Box::new(app_module.resolve_ref()),
        )
        .await,
    )
}

fn parse<T: DeserializeOwned>(
    params: Option<serde_json::Value>,
) -> Result<T, JsonRpcError> {
    serde_json::from_value(params.ok_or_else(params_required)?)
        .map_err(|e| invalid_params(e.to_string()))
}

fn to_result<T: Serialize>(
    outcome: Result<T, mycelium_base::utils::errors::MappedErrors>,
) -> Result<serde_json::Value, JsonRpcError> {
    let value = outcome.map_err(mapped_errors_to_jsonrpc_error)?;

    serde_json::to_value(value).map_err(|e| JsonRpcError {
        code: types::codes::INTERNAL_ERROR,
        message: e.to_string(),
        data: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::openrpc::{generate_openrpc_spec, OpenRpcSpecConfig};

    #[test]
    fn the_openrpc_document_lists_the_federation_methods() {
        let spec = generate_openrpc_spec(&OpenRpcSpecConfig::default());
        let names: Vec<_> = spec["methods"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|method| method["name"].as_str())
            .collect();

        for name in [
            method_names::FEDERATION_PEERS_LIST,
            method_names::FEDERATION_PEERS_QUARANTINE,
            method_names::FEDERATION_PEERS_RELEASE,
            method_names::FEDERATION_SET_ENABLED,
        ] {
            assert!(names.contains(&name), "{name} missing");
        }
    }
}
