use crate::dtos::MyceliumProfileData;

use crate::models::active_backend_modules::SqlAppModule;
use actix_web::{get, post, web, HttpResponse, Responder};
use myc_core::{
    domain::dtos::{
        security_group::PermissionedRole, token::PublicConnectionStringInfo,
    },
    models::{AccountLifeCycle, ServiceNotificationsConfig},
    use_cases::role_scoped::beginner::token::{
        create_connection_string, create_notification_connection_string,
        list_my_connection_strings,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, handle_mapped_error,
    },
};
use myc_mem_db::repositories::MemDbAppModule;
use serde::{Deserialize, Serialize};
use shaku::HasComponent;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_connection_string_url)
        .service(create_notification_connection_string_url)
        .service(list_my_connection_strings_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenBody {
    /// The expiration time of the token
    ///
    /// The expiration time of the token in seconds.
    ///
    expiration: i64,

    /// The name of the token
    ///
    /// The name of the token.
    ///
    name: String,

    /// A single tenant ID
    ///
    /// If specified, the actions allowed by the token will be scoped to the
    /// tenant. If not specified, the actions allowed by the token will be
    /// scoped to the user profile.
    ///
    tenant_id: Option<Uuid>,

    /// A single service account ID
    ///
    /// If specified, the actions allowed by the token will be scoped to the
    /// service account. Service account should be a subscription account,
    /// tenant management account, or role scoped account. If not specified, the
    /// actions allowed by the token will be scoped to the user profile.
    ///
    service_account_id: Option<Uuid>,

    /// A list of roles
    ///
    /// If specified, the actions allowed by the token will be scoped to the
    /// roles. If not specified, the actions allowed by the token will be
    /// scoped to the user profile.
    ///
    roles: Option<Vec<PermissionedRole>>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateNotificationTokenBody {
    /// The expiration time of the token
    ///
    /// The expiration time of the token in seconds. It must be positive and
    /// not above the instance `maxConnectionStringTtlInDays`.
    ///
    expiration: i64,

    /// The name of the token
    name: String,

    /// A single tenant ID
    ///
    /// The tenant the token is bound to. Required when the service
    /// `recipientPolicy` is `tenantMembers`.
    ///
    tenant_id: Option<Uuid>,

    /// The service sender tag
    ///
    /// The `senderTag` of the service allowed to send emails with the token.
    ///
    sender_tag: String,
}

#[derive(Serialize, ToSchema, ToResponse)]
#[response(description = "Token created")]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenResponse {
    connection_string: String,
}

/// Create Connection String
///
/// This action creates a connection string that is associated with the user
/// account. The connection string has the same permissions of the user account.
///
#[utoipa::path(
    post,
    operation_id = "create_connection_string",
    request_body = CreateTokenBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Token created.",
            body = CreateTokenResponse,
        ),
    ),
)]
#[post("")]
pub async fn create_connection_string_url(
    body: web::Json<CreateTokenBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match create_connection_string(
        profile.to_profile(),
        body.name.to_owned(),
        body.expiration.to_owned(),
        body.tenant_id.to_owned(),
        body.service_account_id.to_owned(),
        body.roles.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(connection_string) => {
            HttpResponse::Ok().json(CreateTokenResponse { connection_string })
        }
        Err(err) => handle_mapped_error(err),
    }
}

/// Create Notification Connection String
///
/// This action creates a connection string a downstream service uses to send
/// emails through the gateway. The connection string carries the
/// `notify:email` scope and the service sender tag, and it is rejected on
/// every other gateway surface.
///
#[utoipa::path(
    post,
    operation_id = "create_notification_connection_string",
    request_body = CreateNotificationTokenBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden. MYC00036: service notifications not \
                enabled. MYC00037: issuer not allowed.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 400,
            description = "Bad request. MYC00035: invalid expiration, or the \
                service requires a tenant.",
            body = HttpJsonResponse,
        ),
        (
            status = 201,
            description = "Token created.",
            body = CreateTokenResponse,
        ),
    ),
)]
#[post("/notifications")]
pub async fn create_notification_connection_string_url(
    body: web::Json<CreateNotificationTokenBody>,
    profile: MyceliumProfileData,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    notifications_settings: web::Data<ServiceNotificationsConfig>,
    sql_app_module: web::Data<SqlAppModule>,
    mem_module: web::Data<MemDbAppModule>,
) -> impl Responder {
    match create_notification_connection_string(
        profile.to_profile(),
        body.name.to_owned(),
        body.expiration.to_owned(),
        body.tenant_id.to_owned(),
        body.sender_tag.to_owned(),
        notifications_settings.get_ref().to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*mem_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(connection_string) => HttpResponse::Created()
            .json(CreateTokenResponse { connection_string }),
        Err(err) => handle_mapped_error(err),
    }
}

/// List My Connection Strings
///
/// This action lists all connection strings for the current user.
///
#[utoipa::path(
    get,
    operation_id = "list_my_connection_strings",
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 200,
            description = "Connection strings listed.",
            body = [PublicConnectionStringInfo],
        ),
    ),
)]
#[get("")]
pub async fn list_my_connection_strings_url(
    profile: MyceliumProfileData,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match list_my_connection_strings(
        profile.to_profile(),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// *
// * The issuance rules are covered in the use case. An in-process call that
// * reaches it needs an authenticated profile and the SQL module, which need
// * a live database, so these tests stop at the HTTP contract.
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::ApiDoc;

    use actix_web::{http::StatusCode, test as actix_test, App};
    use utoipa::OpenApi;

    #[test]
    fn notification_body_parses_from_camel_case() {
        let tenant_id = Uuid::new_v4();
        let body: CreateNotificationTokenBody =
            serde_json::from_value(serde_json::json!({
                "name": "token",
                "expiration": 3600,
                "tenantId": tenant_id,
                "senderTag": "service-01",
            }))
            .unwrap();

        assert_eq!(body.tenant_id, Some(tenant_id));
        assert_eq!(body.sender_tag, "service-01");
    }

    #[test]
    fn notification_body_rejects_general_token_fields() {
        let result = serde_json::from_value::<CreateNotificationTokenBody>(
            serde_json::json!({
                "name": "token",
                "expiration": 3600,
                "senderTag": "service-01",
                "serviceAccountId": Uuid::new_v4(),
            }),
        );

        assert!(result.is_err());
    }

    #[actix_web::test]
    async fn notification_route_is_mounted_and_fails_without_credentials() {
        let app = actix_test::init_service(
            App::new().service(web::scope("/tokens").configure(configure)),
        )
        .await;

        let request = actix_test::TestRequest::post()
            .uri("/tokens/notifications")
            .set_json(serde_json::json!({
                "name": "token",
                "expiration": 3600,
                "senderTag": "service-01",
            }))
            .to_request();

        let response = actix_test::call_service(&app, request).await;

        // No credentials nor modules are registered here, so the request
        // must reach the route (not 404) and must not issue a token.
        assert_ne!(response.status(), StatusCode::NOT_FOUND);
        assert!(!response.status().is_success());
    }

    #[test]
    fn notification_route_is_documented_with_its_error_statuses() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();

        let operation = spec["paths"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(path, _)| path.ends_with("/tokens/notifications"))
            .map(|(_, item)| &item["post"])
            .expect("route missing from the OpenAPI spec");

        for status in ["201", "400", "401", "403"] {
            assert!(operation["responses"][status].is_object(), "{status}");
        }
    }
}
