use std::collections::HashMap;

use actix_web::{
    delete, get, post, put, web, HttpRequest, HttpResponse, Responder,
};
use myc_http_tools::{
    dtos::gateway_profile_data::GatewayProfileData, Permission, Profile,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

/// Health check
///
/// Returns a 200 status code if the service is healthy.
///
#[utoipa::path(
    get,
    responses(
        (
            status = 200,
            description = "Success.",
            body = String,
        ),
    ),
    tags = ["Health"],
)]
#[get("/health")]
pub(crate) async fn health() -> impl Responder {
    tracing::debug!("health check");

    HttpResponse::Ok().body("success")
}

/// Public endpoint
///
/// Returns a 200 status code with no restrictions.
///
#[utoipa::path(
    get,
    responses(
        (
            status = 200,
            description = "Success.",
            body = String,
        ),
    ),
    tags = ["Public"],
)]
#[get("/public")]
pub(crate) async fn public() -> impl Responder {
    tracing::debug!("public");

    HttpResponse::Ok().body("success")
}

/// Protected endpoint
///
/// This endpoint is protected and expects to receive the mycelium profile.
/// The same profile is returned in the response. Use this endpoint to test the
/// profile injection when no restrictions should be applied.
///
#[utoipa::path(
    get,
    responses(
        (
            status = 200,
            description = "Success.",
            body = Profile,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = String,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = String,
        ),
    ),
    tags = ["Profile"],
)]
#[get("/protected")]
pub(crate) async fn protected(profile: GatewayProfileData) -> impl Responder {
    let profile = profile.to_profile();

    HttpResponse::Ok().json(profile)
}

/// Protected endpoint by roles
///
/// This endpoint is protected by roles and expects to receive the mycelium
/// profile in the request header. The profile is filtered by the role
/// provided in the request path parameters.
///
#[utoipa::path(
    get,
    params(
        (
            "role" = String,
            Path,
            description = "The role name."
        ),
    ),
    responses(
        (
            status = 200,
            description = "Success.",
            body = Profile,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = String,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = String,
        ),
    ),
    tags = ["Profile", "Roles"],
)]
#[get("/protected/roles/{role}")]
pub(crate) async fn protected_by_role(
    profile: GatewayProfileData,
    params: web::Path<String>,
) -> impl Responder {
    tracing::debug!("protected-by-role: {:?}", profile);

    let profile = profile.to_profile();

    let related_accounts = match profile
        .with_roles(vec![params.into_inner()])
        .get_related_account_or_error()
    {
        Ok(related_accounts) => related_accounts,
        Err(err) => {
            tracing::error!("error getting related accounts: {:?}", err);

            return HttpResponse::InternalServerError().body("error");
        }
    };

    tracing::debug!("related_accounts: {:?}", related_accounts);

    HttpResponse::Ok().json(profile)
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedByRolesWithPermissionParams {
    permission: Permission,
}

/// Protected endpoint by newbie role
///
/// This endpoint is protected by newbie role and expects to receive the
/// mycelium profile in the request header. The profile is filtered by the
/// roles provided in the request query parameters and the permission provided
/// in the request query parameters.
///
#[utoipa::path(
    get,
    params(
        ProtectedByRolesWithPermissionParams,
        (
            "role" = String,
            Path,
            description = "The role name."
        ),
    ),
    responses(
        (
            status = 200,
            description = "Success.",
            body = Profile,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = String,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = String,
        ),
    ),
    tags = ["Profile", "Permission"],
)]
#[get("/protected/roles/{role}/with-permission")]
pub(crate) async fn protected_by_role_with_permission(
    profile: GatewayProfileData,
    path: web::Path<String>,
    params: web::Query<ProtectedByRolesWithPermissionParams>,
) -> impl Responder {
    tracing::debug!("protected-by-role-with-permission: {:?}", profile);

    let mut profile = profile.to_profile();

    match params.permission {
        Permission::Read => {
            profile = profile.with_read_access();
        }
        Permission::Write => {
            profile = profile.with_write_access();
        }
    }

    let related_accounts = match profile
        .with_roles(vec![path.into_inner()])
        .get_related_account_or_error()
    {
        Ok(related_accounts) => related_accounts,
        Err(err) => {
            tracing::error!("error getting related accounts: {:?}", err);

            return HttpResponse::InternalServerError().body("error");
        }
    };

    tracing::debug!("related_accounts: {:?}", related_accounts);

    HttpResponse::Ok().json(profile)
}

/// Protected endpoint by service token with scope
///
/// This endpoint is protected by service token with scope and expects to
/// receive the scope by request header. Roles from request query parameters
/// used to filter the scope.
///
/// WARNING: This endpoint is not implemented yet. It is a placeholder for
/// future use.
///
#[utoipa::path(
    get,
    params(
        ProtectedByRolesWithPermissionParams,
        (
            "role" = String,
            Path,
            description = "The role name."
        ),
        (
            "x-mycelium-scope" = String,
            Header,
            description = "The scope to be used to filter the profile."
        ),
    ),
    responses(
        (
            status = 200,
            description = "Success.",
            body = HashMap<String, String>,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = String,
        ),
    ),
    tags = ["ServiceToken", "Scope"],
)]
#[get("/protected/role/{role}/with-scope")]
pub(crate) async fn protected_by_service_token_with_scope(
    _req: HttpRequest,
    _path: web::Path<String>,
    _params: web::Query<ProtectedByRolesWithPermissionParams>,
) -> impl Responder {
    return HttpResponse::NotImplemented().body(
        "This endpoint is not implemented yet. It is a placeholder for future use.",
    );
}

/// Expects headers
///
/// Returns a 200 status code if the service is ok. This endpoint expects to
/// receive any headers and exists for testing purposes.
///
#[utoipa::path(
    get,
    responses(
        (
            status = 200,
            description = "Success.",
            body = HashMap<String, String>,
        ),
    ),
    tags = ["ServiceToken"],
)]
#[get("/protected/expects-headers")]
pub(crate) async fn expects_headers(req: HttpRequest) -> impl Responder {
    let headers = req
        .headers()
        .iter()
        .map(|(key, value)| {
            (key.to_string(), value.to_str().unwrap_or("").to_string())
        })
        .collect::<HashMap<String, String>>();

    HttpResponse::Ok().json(headers)
}

/// Webhook body
///
/// The body used to propagate the webhook event.
///
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebHookBody {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    created: String,
}

/// Account created webhook
///
/// Returns a 200 status code if the service is ok. This endpoint is a webhook
/// and expects to receive the webhook body.
///
#[utoipa::path(
    post,
    responses(
        (
            status = 200,
            description = "Success.",
            body = WebHookBody,
        ),
    ),
    tags = ["Webhooks"],
)]
#[post("/webhooks/account-created")]
pub(crate) async fn account_created_webhook(
    body: web::Json<WebHookBody>,
) -> impl Responder {
    tracing::debug!("account-created-webhook: {:?}", body);

    HttpResponse::Ok().json(body)
}

/// Account updated webhook
///
/// Returns a 200 status code if the service is ok. This endpoint is a webhook
/// and expects to receive the webhook body.
///
#[utoipa::path(
    put,
    responses(
        (
            status = 200,
            description = "Success.",
            body = WebHookBody,
        ),
    ),
    tags = ["Webhooks"],
)]
#[put("/webhooks/account-updated")]
pub(crate) async fn account_updated_webhook(
    body: web::Json<WebHookBody>,
) -> impl Responder {
    tracing::debug!("account-updated-webhook: {:?}", body);

    HttpResponse::Ok().json(body)
}

/// Account deleted webhook
///
/// Returns a 200 status code if the service is ok. This endpoint is a webhook
/// and expects to receive the webhook body.
///
#[utoipa::path(
    delete,
    responses(
        (
            status = 200,
            description = "Success.",
            body = WebHookBody,
        ),
    ),
    tags = ["Webhooks"],
)]
#[delete("/webhooks/account-deleted")]
pub(crate) async fn account_deleted_webhook(
    body: web::Json<WebHookBody>,
) -> impl Responder {
    tracing::debug!("account-deleted-webhook: {:?}", body);

    HttpResponse::Ok().json(body)
}

/// Test for authorization header token
///
/// Mycelium API Gateway provide the injection of static secrets to the
/// downstream route to protect the route from unauthenticated access. This
/// route should be used to test the authorization header token injection.
///
#[utoipa::path(
    get,
    params(
        (
            "Authorization" = String,
            Header,
            description = "The authorization header."
        ),
    ),
    responses(
        (
            status = 200,
            description = "Success.",
            body = HashMap<String, String>,
        ),
    ),
    tags = ["Secrets"],
)]
#[get("/secrets/authorization-header")]
async fn test_authorization_header(req: HttpRequest) -> impl Responder {
    let header = req
        .headers()
        .get("authorization")
        .map(|value| value.to_str().unwrap_or("").to_string())
        .unwrap_or("".to_string());

    tracing::debug!("test_authorization_header_token: {:?}", header);

    HttpResponse::Ok().json(json!({ "authorization": header }))
}

#[derive(Debug, Deserialize, Serialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct QueryParameterToken {
    #[allow(dead_code)]
    token: String,
}

/// Test for query parameter token
///
/// Mycelium API Gateway provide the injection of static secrets to the
/// downstream route to protect the route from unauthenticated access. This
/// route should be used to test the query parameter token injection.
///
#[utoipa::path(
    get,
    params(
        QueryParameterToken,
    ),
    responses(
        (
            status = 200,
            description = "Success.",
            body = HashMap<String, String>,
        ),
    ),
    tags = ["Secrets"],
)]
#[get("/secrets/query-parameter-token")]
async fn test_query_parameter_token(
    params: web::Query<QueryParameterToken>,
) -> impl Responder {
    let token = params.token.clone();

    tracing::debug!("test_query_parameter_token: {:?}", token);

    HttpResponse::Ok().json(json!({ "token": token }))
}
