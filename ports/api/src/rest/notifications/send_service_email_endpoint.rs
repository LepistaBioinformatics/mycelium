use super::answer_quota_outcome::{
    answer_quota_outcome, SendServiceEmailResponse,
};
use crate::{
    middleware::fetch_notification_connection_string_from_request,
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{post, web, HttpRequest, HttpResponse, ResponseError};
use myc_core::{
    domain::dtos::{
        native_error_codes::NativeErrorCodes,
        send_service_email_request::SendServiceEmailRequest,
    },
    models::{AccountLifeCycle, ServiceNotificationsConfig, ServiceTemplates},
    use_cases::service::notifications::send_service_email,
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use myc_mem_db::repositories::MemDbAppModule;
use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use shaku::HasComponent;

/// Send Service Email
///
/// Render an operator-deployed template of the calling service and queue
/// one email per recipient. The caller authenticates ONLY with a
/// notification connection string (`notify:email` scope) in the
/// `x-mycelium-connection-string` header; JWTs and general connection
/// strings are refused. The service is the one bound to the string, never
/// one named in the body.
///
/// **The connection string is a long-lived secret. Never log it, never put
/// it in a URL, and never echo it in error reports.** Treat recipient
/// addresses and template variables as personal data too.
///
/// The body accepts only `to`, `template`, `variables` and `locale`. Sender,
/// reply address, subject, headers and HTML are always set by the gateway.
/// Delivery is asynchronous: 202 means queued, not delivered.
///
#[utoipa::path(
    post,
    operation_id = "send_service_email",
    request_body = SendServiceEmailRequest,
    responses(
        (
            status = 202,
            description = "Queued, one message per recipient.",
            body = SendServiceEmailResponse,
        ),
        (
            status = 401,
            description = "Missing, tampered, expired, revoked or general \
                connection string. MYC00030/MYC00031/MYC00032: invalid \
                signature. MYC00034: the string lacks the notify:email \
                scope.",
            body = HttpJsonResponse,
        ),
        (
            status = 403,
            description = "MYC00036: service notifications not enabled. \
                MYC00037: the string creator is no longer an allowed \
                issuer. MYC00039: a recipient is not allowed by the \
                service recipient policy.",
            body = HttpJsonResponse,
        ),
        (
            status = 422,
            description = "MYC00038: unknown body field, template not \
                declared or not found, missing or non-scalar variable, \
                variables over 8192 bytes, invalid recipient or recipient \
                count out of range.",
            body = HttpJsonResponse,
        ),
        (
            status = 429,
            description = "MYC00040: hourly quota of the service exceeded. \
                The `Retry-After` header gives the seconds to wait.",
            body = HttpJsonResponse,
        ),
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
    ),
    security(("ConnectionString" = [])),
)]
#[post("/email")]
pub async fn send_service_email_url(
    req: HttpRequest,
    body: web::Bytes,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    notifications_settings: web::Data<ServiceNotificationsConfig>,
    service_templates: web::Data<Option<ServiceTemplates>>,
    sql_app_module: web::Data<SqlAppModule>,
    mem_module: web::Data<MemDbAppModule>,
) -> HttpResponse {
    //
    // Authenticate before reading the body, so an unauthenticated caller
    // learns nothing about the request shape.
    //
    let (connection_string_id, connection_string) =
        match fetch_notification_connection_string_from_request(req).await {
            Ok(verified) => verified,
            Err(err) => return err.error_response(),
        };

    let request = match parse_send_request(&body) {
        Ok(request) => request,
        Err(err) => return handle_mapped_error(err),
    };

    let Some(service_templates) = service_templates.get_ref() else {
        return handle_mapped_error(notifications_not_enabled());
    };

    match send_service_email(
        connection_string,
        connection_string_id,
        request,
        notifications_settings.get_ref(),
        service_templates,
        life_cycle_settings.get_ref(),
        Box::new(&*mem_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(outcome) => answer_quota_outcome(outcome),
        Err(err) => handle_mapped_error(err),
    }
}

/// Parse the body, refusing unknown fields with MYC00038 (422)
///
/// The parser message is not echoed back: it may quote request values.
fn parse_send_request(
    body: &[u8],
) -> Result<SendServiceEmailRequest, MappedErrors> {
    serde_json::from_slice(body).map_err(|_| {
        use_case_err(
            "Invalid request body: only to, template, variables and locale \
             are accepted",
        )
        .with_code(NativeErrorCodes::MYC00038)
        .with_exp_true()
    })
}

fn notifications_not_enabled() -> MappedErrors {
    use_case_err("Service notifications not enabled")
        .with_code(NativeErrorCodes::MYC00036)
        .with_exp_true()
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::{
        configure,
        send_service_email_test_doubles::{
            mem_module, sql_module, StoredState,
        },
    };
    use crate::{
        middleware::connection_string_test_fixtures::{
            general_connection_string, life_cycle,
        },
        openapi::ApiDoc,
    };

    use actix_web::{
        dev::ServiceResponse,
        http::{header::RETRY_AFTER, StatusCode},
        test as actix_test, web, App,
    };
    use base64::{engine::general_purpose, Engine};
    use chrono::{Duration, Local};
    use myc_core::{
        domain::dtos::{
            email::Email,
            notification_dispatch::QuotaOutcome,
            profile::{Owner, Profile},
            service::Service,
            token::{
                MultiTypeMeta, NotificationScope, Token, UserAccountScope,
            },
        },
        models::{ServiceNotificationsConfig, ServiceTemplates},
    };
    use myc_http_tools::settings::DEFAULT_CONNECTION_STRING_KEY;
    use serde_json::{json, Value};
    use std::{fs, path::PathBuf};
    use utoipa::OpenApi;
    use uuid::Uuid;

    const SENDER_TAG: &str = "service-01";
    const ISSUER_ROLE: &str = "service-01-admin";
    const OWNER_EMAIL: &str = "owner@example.com";

    // ? -----------------------------------------------------------------------
    // ? Fixtures
    // ? -----------------------------------------------------------------------

    struct TemplatesDir(PathBuf);

    impl TemplatesDir {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("myc-send-endpoint-{}", Uuid::new_v4()));
            let service = root.join(SENDER_TAG).join("en-us");

            fs::create_dir_all(&service).unwrap();
            fs::write(service.join("welcome.jinja"), "<p>Hi {{ name }}</p>")
                .unwrap();
            fs::write(service.join("welcome.subject"), "Welcome").unwrap();

            Self(root)
        }
    }

    impl Drop for TemplatesDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn service(enabled: bool) -> Service {
        let mut service = json!({
            "name": SENDER_TAG,
            "host": "localhost:8080",
            "protocol": "http",
            "routes": [],
            "healthCheckPath": "/health",
        });

        if enabled {
            service["notifications"] = json!({
                "senderTag": SENDER_TAG,
                "displayName": "Service 01",
                "issuerRoles": [ISSUER_ROLE],
                "recipientPolicy": "any",
                "templates": ["welcome"],
            });
        }

        serde_json::from_value(service).unwrap()
    }

    fn profile(account_id: Uuid) -> Profile {
        Profile::new(
            vec![Owner {
                id: Uuid::new_v4(),
                email: OWNER_EMAIL.to_string(),
                first_name: None,
                last_name: None,
                username: None,
                is_principal: true,
            }],
            account_id,
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    /// A signed notification string and the row the database holds for it.
    struct IssuedString {
        header: String,
        stored: Token,
        account_id: Uuid,
        tenant_id: Uuid,
    }

    async fn issued_string() -> IssuedString {
        let account_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let scope = UserAccountScope::new_notification(
            account_id,
            Local::now() + Duration::days(1),
            Some(tenant_id),
            vec![NotificationScope::NotifyEmail],
            SENDER_TAG.to_string(),
            life_cycle(),
        )
        .await
        .unwrap();

        let meta = serde_json::from_value(json!({
            "scope": scope,
            "accountId": account_id,
            "email": Email::from_string(OWNER_EMAIL.to_string()).unwrap(),
            "token": "stored",
        }))
        .unwrap();

        IssuedString {
            header: scope.to_string(),
            stored: Token::new(
                Some(11),
                Local::now() + Duration::days(1),
                MultiTypeMeta::UserAccountConnectionString(meta),
            ),
            account_id,
            tenant_id,
        }
    }

    fn state(issued: &IssuedString, quota: QuotaOutcome) -> StoredState {
        StoredState {
            token: Some(issued.stored.to_owned()),
            profile: Some(profile(issued.account_id)),
            role: ISSUER_ROLE.to_string(),
            tenant_id: issued.tenant_id,
            quota,
        }
    }

    fn accepted() -> QuotaOutcome {
        QuotaOutcome::Accepted(vec![])
    }

    fn valid_body() -> Value {
        json!({
            "to": [OWNER_EMAIL, "second@example.com"],
            "template": "welcome",
            "variables": { "name": "Ana" },
        })
    }

    async fn send(
        state: StoredState,
        services: Vec<Service>,
        connection_string: &str,
        body: Value,
    ) -> ServiceResponse {
        let templates_dir = TemplatesDir::new();
        let templates =
            ServiceTemplates::load(&templates_dir.0, "en-us").unwrap();

        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(life_cycle()))
                .app_data(web::Data::new(ServiceNotificationsConfig::default()))
                .app_data(web::Data::new(Some(templates)))
                .app_data(web::Data::new(sql_module(state)))
                .app_data(web::Data::new(mem_module(services)))
                .service(
                    web::scope("/_adm/notifications").configure(configure),
                ),
        )
        .await;

        let request = actix_test::TestRequest::post()
            .uri("/_adm/notifications/email")
            .insert_header((DEFAULT_CONNECTION_STRING_KEY, connection_string))
            .set_json(body)
            .to_request();

        actix_test::call_service(&app, request).await
    }

    async fn assert_status(
        response: ServiceResponse,
        status: StatusCode,
        code: Option<&str>,
    ) {
        assert_eq!(response.status(), status);

        let Some(code) = code else {
            return;
        };

        let body =
            String::from_utf8(actix_test::read_body(response).await.to_vec())
                .unwrap();

        assert!(body.contains(code), "{body}");
    }

    /// Rewrite the raw beans of an encoded connection string.
    fn tamper(connection_string: &str, from: &str, to: &str) -> String {
        let raw = general_purpose::STANDARD.decode(connection_string).unwrap();
        let raw = String::from_utf8(raw).unwrap().replace(from, to);

        general_purpose::STANDARD.encode(raw)
    }

    // ? -----------------------------------------------------------------------
    // ? 202
    // ? -----------------------------------------------------------------------

    #[actix_web::test]
    async fn queues_and_answers_202_with_one_id_per_recipient() {
        let issued = issued_string().await;

        let response = send(
            state(&issued, accepted()),
            vec![service(true)],
            &issued.header,
            valid_body(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let body: Value = actix_test::read_body_json(response).await;
        let ids = body["messageIds"].as_array().unwrap();

        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }

    // ? -----------------------------------------------------------------------
    // ? 401
    // ? -----------------------------------------------------------------------

    #[actix_web::test]
    async fn general_string_is_401() {
        let issued = issued_string().await;
        let general = general_connection_string(&life_cycle()).await;

        let response = send(
            state(&issued, accepted()),
            vec![service(true)],
            &general,
            valid_body(),
        )
        .await;

        assert_status(response, StatusCode::UNAUTHORIZED, Some("MYC00034"))
            .await;
    }

    #[actix_web::test]
    async fn tampered_string_is_401() {
        let issued = issued_string().await;
        let tampered =
            tamper(&issued.header, "svc=service-01", "svc=service-02");

        let response = send(
            state(&issued, accepted()),
            vec![service(true)],
            &tampered,
            valid_body(),
        )
        .await;

        assert_status(response, StatusCode::UNAUTHORIZED, Some("MYC00032"))
            .await;
    }

    #[actix_web::test]
    async fn revoked_or_expired_string_is_401() {
        let issued = issued_string().await;
        let mut revoked = state(&issued, accepted());
        revoked.token = None;

        let response =
            send(revoked, vec![service(true)], &issued.header, valid_body())
                .await;

        assert_status(response, StatusCode::UNAUTHORIZED, None).await;
    }

    #[actix_web::test]
    async fn bearer_token_alone_is_401() {
        let issued = issued_string().await;
        let templates_dir = TemplatesDir::new();

        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(life_cycle()))
                .app_data(web::Data::new(ServiceNotificationsConfig::default()))
                .app_data(web::Data::new(Some(
                    ServiceTemplates::load(&templates_dir.0, "en-us").unwrap(),
                )))
                .app_data(web::Data::new(sql_module(state(
                    &issued,
                    accepted(),
                ))))
                .app_data(web::Data::new(mem_module(vec![service(true)])))
                .service(
                    web::scope("/_adm/notifications").configure(configure),
                ),
        )
        .await;

        let request = actix_test::TestRequest::post()
            .uri("/_adm/notifications/email")
            .insert_header(("Authorization", "Bearer some.jwt.token"))
            .set_json(valid_body())
            .to_request();

        let response = actix_test::call_service(&app, request).await;

        assert_status(response, StatusCode::UNAUTHORIZED, None).await;
    }

    #[actix_web::test]
    async fn authentication_runs_before_the_body_is_read() {
        let issued = issued_string().await;
        let general = general_connection_string(&life_cycle()).await;

        let response = send(
            state(&issued, accepted()),
            vec![service(true)],
            &general,
            json!({ "from": "attacker@evil.example" }),
        )
        .await;

        assert_status(response, StatusCode::UNAUTHORIZED, Some("MYC00034"))
            .await;
    }

    // ? -----------------------------------------------------------------------
    // ? 403
    // ? -----------------------------------------------------------------------

    #[actix_web::test]
    async fn service_disabled_since_issuance_is_403() {
        let issued = issued_string().await;

        let response = send(
            state(&issued, accepted()),
            vec![service(false)],
            &issued.header,
            valid_body(),
        )
        .await;

        assert_status(response, StatusCode::FORBIDDEN, Some("MYC00036")).await;
    }

    #[actix_web::test]
    async fn revoked_issuer_role_is_403() {
        let issued = issued_string().await;
        let mut revoked_role = state(&issued, accepted());
        revoked_role.role = "another-role".to_string();

        let response = send(
            revoked_role,
            vec![service(true)],
            &issued.header,
            valid_body(),
        )
        .await;

        assert_status(response, StatusCode::FORBIDDEN, Some("MYC00037")).await;
    }

    // ? -----------------------------------------------------------------------
    // ? 422
    // ? -----------------------------------------------------------------------

    #[actix_web::test]
    async fn unknown_body_field_is_422() {
        let issued = issued_string().await;
        let mut body = valid_body();
        body["from"] = json!("attacker@evil.example");

        let response = send(
            state(&issued, accepted()),
            vec![service(true)],
            &issued.header,
            body,
        )
        .await;

        assert_status(
            response,
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("MYC00038"),
        )
        .await;
    }

    #[actix_web::test]
    async fn injected_recipient_is_422() {
        let issued = issued_string().await;
        let mut body = valid_body();
        body["to"] =
            json!(["victim@allowed.example\r\nBcc: other@evil.example"]);

        let response = send(
            state(&issued, accepted()),
            vec![service(true)],
            &issued.header,
            body,
        )
        .await;

        assert_status(
            response,
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("MYC00038"),
        )
        .await;
    }

    // ? -----------------------------------------------------------------------
    // ? 429
    // ? -----------------------------------------------------------------------

    #[actix_web::test]
    async fn exceeded_quota_is_429_with_retry_after() {
        let issued = issued_string().await;
        let exceeded = QuotaOutcome::Exceeded {
            retry_after_secs: 1800,
        };

        let response = send(
            state(&issued, exceeded),
            vec![service(true)],
            &issued.header,
            valid_body(),
        )
        .await;

        assert_eq!(response.headers().get(RETRY_AFTER).unwrap(), "1800");
        assert_status(
            response,
            StatusCode::TOO_MANY_REQUESTS,
            Some("MYC00040"),
        )
        .await;
    }

    // ? -----------------------------------------------------------------------
    // ? OpenAPI
    // ? -----------------------------------------------------------------------

    #[test]
    fn endpoint_is_documented_with_its_statuses_and_warning() {
        let spec = serde_json::to_value(ApiDoc::openapi()).unwrap();
        let operation = &spec["paths"]["/_adm/notifications/email"]["post"];

        for status in ["202", "401", "403", "422", "429"] {
            assert!(operation["responses"][status].is_object(), "{status}");
        }

        assert!(operation["description"]
            .as_str()
            .unwrap()
            .contains("Never log it"));
    }
}
