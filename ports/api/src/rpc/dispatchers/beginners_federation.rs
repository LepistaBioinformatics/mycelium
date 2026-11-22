use super::{
    super::{
        errors::{invalid_params, mapped_errors_to_jsonrpc_error},
        method_names,
        params::{LinkParams, StartLinkParams},
        types::{self, JsonRpcError},
    },
    federation::{method_not_found, parse, to_result},
    revoke_identity_link::revoke_identity_link,
};
use crate::{
    dtos::MyceliumProfileData, federation::PreparedFederation,
    middleware::check_credentials_with_multi_identity_provider,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{web, HttpRequest};
use chrono::{DateTime, Duration, Local};
use myc_core::{
    domain::{
        dtos::{profile::Owner, written_by::WrittenBy},
        entities::InstanceSettingsFetching,
    },
    use_cases::federation::{
        is_federation_enabled, list_identity_links, list_linkable_peers,
        start_link, LinkRevoker,
    },
};
use myc_http_tools::settings::FEDERATION_LINK_START_PATH;
use serde::Serialize;
use shaku::HasComponent;
use uuid::Uuid;

/// A link initiation as returned to the user's application (FED-42)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LinkInitiation {
    request_id: Uuid,
    initiation_url: String,
    expires_at: DateTime<Local>,
}

/// `beginners.federation.*`: a user's own identity links (FED-42, FED-43,
/// FED-47)
pub async fn dispatch_beginners_federation(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    req: Option<&HttpRequest>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    let prepared = enabled_federation(app_module, req).await?;

    // Every method needs a signed-in account owner: the RPC endpoint itself
    // lets anonymous calls through with an empty profile
    let owner = requesting_owner(profile, req).await?;

    match method {
        method_names::BEGINNERS_FEDERATION_LIST_PEERS => to_result(
            list_linkable_peers(Box::new(app_module.resolve_ref())).await,
        ),
        method_names::BEGINNERS_FEDERATION_START_LINK => {
            let params = parse::<StartLinkParams>(params)?;
            initiate(app_module, prepared, owner, params.instance_id).await
        }
        method_names::BEGINNERS_FEDERATION_LIST_LINKS => to_result(
            list_identity_links(
                Some(owner.id),
                Box::new(app_module.resolve_ref()),
            )
            .await,
        ),
        method_names::BEGINNERS_FEDERATION_REVOKE_LINK => {
            let link_id = parse::<LinkParams>(params)?.link_id;
            let author =
                WrittenBy::new_from_user_with_email(owner.id, &owner.email);
            revoke_identity_link(
                app_module,
                req,
                link_id,
                LinkRevoker::User(owner.id),
                author,
            )
            .await
        }
        _ => Err(method_not_found(method)),
    }
}

/// The prepared federation, refused while absent or switched off
async fn enabled_federation<'a>(
    app_module: &web::Data<SqlAppModule>,
    req: Option<&'a HttpRequest>,
) -> Result<&'a PreparedFederation, JsonRpcError> {
    let prepared = req
        .and_then(|req| req.app_data::<web::Data<Option<PreparedFederation>>>())
        .and_then(|data| data.get_ref().as_ref());
    let settings: &dyn InstanceSettingsFetching = app_module.resolve_ref();

    match (prepared, is_federation_enabled(Box::new(settings)).await) {
        (Some(prepared), Ok(true)) => Ok(prepared),
        (_, Err(err)) => Err(mapped_errors_to_jsonrpc_error(err)),
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: "federation is not enabled".to_string(),
            data: None,
        }),
    }
}

/// The account owner the bearer token belongs to
async fn requesting_owner(
    profile: &MyceliumProfileData,
    req: Option<&HttpRequest>,
) -> Result<Owner, JsonRpcError> {
    let req = req.ok_or_else(|| invalid_params("Request context required"))?;
    let (email, _) =
        check_credentials_with_multi_identity_provider(req.to_owned())
            .await
            .map_err(|err| invalid_params(err.to_string()))?;
    let email = email.email();

    profile
        .to_profile()
        .owners
        .into_iter()
        .find(|owner| owner.email.eq_ignore_ascii_case(&email))
        .ok_or_else(|| invalid_params("no account owner for this token"))
}

async fn initiate(
    app_module: &web::Data<SqlAppModule>,
    prepared: &PreparedFederation,
    owner: Owner,
    peer_instance_id: Uuid,
) -> Result<serde_json::Value, JsonRpcError> {
    let lifetime =
        Duration::seconds(prepared.config.link_request_ttl_secs as i64);
    let (request, secret) = start_link(
        owner.id,
        peer_instance_id,
        lifetime,
        Box::new(app_module.resolve_ref()),
        Box::new(app_module.resolve_ref()),
        Box::new(app_module.resolve_ref()),
    )
    .await
    .map_err(mapped_errors_to_jsonrpc_error)?;

    let base_url = prepared.config.base_url.trim_end_matches('/');
    to_result(Ok(LinkInitiation {
        initiation_url: format!(
            "{base_url}{FEDERATION_LINK_START_PATH}?request={}.{}",
            request.id,
            secret.as_str()
        ),
        request_id: request.id,
        expires_at: request.expires_at,
    }))
}
