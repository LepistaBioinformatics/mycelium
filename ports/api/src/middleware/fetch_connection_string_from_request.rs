use crate::dtos::MyceliumConnectionStringData;

use crate::models::active_backend_modules::SqlAppModule;
use actix_web::{web, HttpRequest};
use myc_core::{
    domain::{
        dtos::{
            native_error_codes::NativeErrorCodes,
            token::{
                MultiTypeMeta, NotificationScope, UserAccountConnectionString,
                UserAccountScope,
            },
        },
        entities::TokenFetching,
    },
    models::AccountLifeCycle,
};
use myc_http_tools::{
    responses::GatewayError, settings::DEFAULT_CONNECTION_STRING_KEY,
};
use mycelium_base::entities::FetchResponseKind;
use shaku::HasComponent;
use tracing::{error, warn};

/// Fetch a general (unscoped) connection string from the request
///
/// This is the single entry point behind every surface that resolves a
/// profile from a connection string: routed downstream requests, admin REST
/// (through `MyceliumProfileData`), JSON-RPC and, by loopback, MCP. Scoped
/// strings (carrying an `SCP` bean, e.g. notification strings) are rejected
/// here with MYC00033, so they never yield a profile on any of them.
///
#[tracing::instrument(name = "fetch_connection_string_from_request", skip_all)]
pub async fn fetch_connection_string_from_request(
    req: HttpRequest,
) -> Result<MyceliumConnectionStringData, GatewayError> {
    let scope = parse_and_verify_connection_string(&req).await?;

    if scope.get_scopes().is_some() {
        warn!(
            connection_string_verification_failed = true,
            code = %NativeErrorCodes::MYC00033,
            "Rejecting scoped connection string on a general surface",
        );

        return Err(GatewayError::Unauthorized(format!(
            "Invalid connection string: {}",
            NativeErrorCodes::MYC00033,
        )));
    }

    let connection_string = fetch_stored_connection_string(&req, scope).await?;

    Ok(MyceliumConnectionStringData::new(connection_string))
}

/// Fetch a notification connection string from the request
///
/// Accepts only strings whose `SCP` bean grants `notify:email`. General
/// strings, and scoped strings without that scope, are rejected with
/// MYC00034.
///
#[allow(dead_code)] // Wired by the notification send endpoint.
#[tracing::instrument(
    name = "fetch_notification_connection_string_from_request",
    skip_all
)]
pub async fn fetch_notification_connection_string_from_request(
    req: HttpRequest,
) -> Result<UserAccountConnectionString, GatewayError> {
    let scope = parse_and_verify_connection_string(&req).await?;

    let grants_notify_email = scope
        .get_scopes()
        .is_some_and(|scopes| scopes.contains(&NotificationScope::NotifyEmail));

    if !grants_notify_email {
        warn!(
            connection_string_verification_failed = true,
            code = %NativeErrorCodes::MYC00034,
            "Rejecting connection string without the notify:email scope",
        );

        return Err(GatewayError::Unauthorized(format!(
            "Invalid connection string: {}",
            NativeErrorCodes::MYC00034,
        )));
    }

    fetch_stored_connection_string(&req, scope).await
}

/// Read the connection string header, parse it and verify its signature
///
/// The scope check runs on the returned scope before the database lookup:
/// the HMAC covers every bean, so the `SCP` bean read here is the one that
/// was issued.
///
async fn parse_and_verify_connection_string(
    req: &HttpRequest,
) -> Result<UserAccountScope, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Fetch connection string from request header
    //
    // Use the `DEFAULT_CONNECTION_STRING_KEY` to fetch the connection string
    // from the request header.
    //
    // ? -----------------------------------------------------------------------

    let connection_string_header =
        match req.headers().get(DEFAULT_CONNECTION_STRING_KEY) {
            Some(value) => value,
            None => {
                return Err(GatewayError::Unauthorized(
                    "Connection string not found in request".to_string(),
                ))
            }
        };

    let connection_string = match connection_string_header.to_str() {
        Ok(value) => value,
        Err(_) => {
            return Err(GatewayError::Unauthorized(
                "Connection string is not a valid string".to_string(),
            ))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Parse scope from connection string
    // ? -----------------------------------------------------------------------

    let scope = match UserAccountScope::try_from(connection_string.to_string())
    {
        Ok(value) => value,
        Err(_) => {
            return Err(GatewayError::Unauthorized(
                "Connection string has invalid scope".to_string(),
            ))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Verify HMAC before touching the database
    //
    // Reject forged or downgraded tokens up-front. Native error codes
    // MYC00030 / MYC00031 / MYC00032 are carried through so logs can be
    // searched by reason. See `22-hmac-key-rotation.md` for the contract.
    //
    // ? -----------------------------------------------------------------------

    let life_cycle = match req.app_data::<web::Data<AccountLifeCycle>>() {
        Some(value) => value,
        None => {
            error!(
                "AccountLifeCycle is not registered as app data; cannot \
                 verify connection-string signature",
            );
            return Err(GatewayError::InternalServerError(
                "Server misconfiguration: HMAC key set unavailable".to_string(),
            ));
        }
    };

    if let Err(err) = scope.verify_signature(life_cycle.get_ref()).await {
        warn!(
            connection_string_verification_failed = true,
            code = %err.code(),
            "Rejecting connection string: {err}",
        );
        return Err(GatewayError::Unauthorized(format!(
            "Invalid connection string: {}",
            err.code(),
        )));
    }

    Ok(scope)
}

/// Fetch the stored connection string matching a verified scope
///
/// Expired, revoked and unknown strings are not found and rejected as
/// unauthorized.
///
async fn fetch_stored_connection_string(
    req: &HttpRequest,
    scope: UserAccountScope,
) -> Result<UserAccountConnectionString, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Build dependencies
    // ? -----------------------------------------------------------------------

    let repo: &dyn TokenFetching = match req
        .app_data::<web::Data<SqlAppModule>>()
    {
        Some(module) => module.resolve_ref(),
        None => {
            error!("Unable to extract profile fetching module from request");

            return Err(GatewayError::InternalServerError(
                "Unexpected error on get profile".to_string(),
            ));
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Extract the connection string from the repo
    // ? -----------------------------------------------------------------------

    let token = match repo.get_connection_string(scope).await {
        Ok(value) => {
            match value {
                FetchResponseKind::Found(token) => token,
                FetchResponseKind::NotFound(msg) => {
                    if let Some(msg) = msg {
                        error!("Connection string not found in the database: {msg}");
                    } else {
                        error!("Connection string not found in the database");
                    }

                    return Err(GatewayError::Unauthorized(
                        "Invalid connection string".to_string(),
                    ));
                }
            }
        }
        Err(err) => {
            return Err(GatewayError::InternalServerError(format!(
                "Unable to fetch connection string: {err}"
            )))
        }
    };

    let meta = match token.meta {
        MultiTypeMeta::UserAccountConnectionString(string) => string,
        _ => {
            error!("Connection string is not a UserAccountConnectionString");

            return Err(GatewayError::InternalServerError(
                "Invalid connection string".to_string(),
            ));
        }
    };

    Ok(meta)
}

/// Signed connection strings and requests shared by the isolation tests of
/// every surface (this middleware, REST, RPC and the routed path).
#[cfg(test)]
pub(crate) mod connection_string_test_fixtures {
    use actix_web::{test::TestRequest, web, HttpRequest};
    use chrono::{Duration, Local};
    use myc_core::{
        domain::dtos::token::{NotificationScope, UserAccountScope},
        models::AccountLifeCycle,
    };
    use myc_http_tools::settings::DEFAULT_CONNECTION_STRING_KEY;
    use uuid::Uuid;

    pub(crate) fn life_cycle() -> AccountLifeCycle {
        serde_json::from_value(serde_json::json!({
            "domainName": "example.com",
            "noreplyEmail": "noreply@example.com",
            "supportEmail": "support@example.com",
            "tokenSecret": "placeholder",
            "hmacSecrets": [{ "version": 1, "secret": "placeholder" }],
        }))
        .unwrap()
    }

    pub(crate) async fn general_connection_string(
        life_cycle: &AccountLifeCycle,
    ) -> String {
        UserAccountScope::new(
            Uuid::new_v4(),
            Local::now() + Duration::days(1),
            None,
            Some(Uuid::new_v4()),
            None,
            life_cycle.to_owned(),
        )
        .await
        .unwrap()
        .to_string()
    }

    pub(crate) async fn notification_connection_string(
        life_cycle: &AccountLifeCycle,
    ) -> String {
        UserAccountScope::new_notification(
            Uuid::new_v4(),
            Local::now() + Duration::days(1),
            Some(Uuid::new_v4()),
            vec![NotificationScope::NotifyEmail],
            "service-01".to_string(),
            life_cycle.to_owned(),
        )
        .await
        .unwrap()
        .to_string()
    }

    /// A request carrying the connection string and the HMAC key set, but
    /// no SQL module: a string that passes every check before the database
    /// lookup ends in `InternalServerError`, one rejected earlier does not.
    pub(crate) fn request_with_connection_string(
        connection_string: &str,
        life_cycle: &AccountLifeCycle,
    ) -> HttpRequest {
        TestRequest::default()
            .insert_header((DEFAULT_CONNECTION_STRING_KEY, connection_string))
            .app_data(web::Data::new(life_cycle.to_owned()))
            .app_data(web::Data::new(
                myc_http_tools::models::ProfileSigningState::Disabled,
            ))
            .to_http_request()
    }
}

// MCP needs no dedicated isolation test: its tool calls forward the
// connection string header to the gateway loopback (REST or JSON-RPC), so a
// scoped string lands on the surfaces covered here and in
// `dtos/mycelium_profile_data.rs`, `rpc/handlers.rs` and
// `fetch_and_inject_profile_from_token_to_forward.rs`.
#[cfg(test)]
mod tests {
    use super::connection_string_test_fixtures::*;
    use super::*;

    use base64::{engine::general_purpose, Engine};

    fn assert_unauthorized_with_code(
        result: Result<impl std::fmt::Debug, GatewayError>,
        code: NativeErrorCodes,
    ) {
        match result {
            Err(GatewayError::Unauthorized(msg)) => {
                assert!(msg.contains(&code.to_string()), "{msg}")
            }
            other => panic!("expected Unauthorized({code}), got {other:?}"),
        }
    }

    fn assert_reached_database_lookup(
        result: Result<impl std::fmt::Debug, GatewayError>,
    ) {
        assert!(
            matches!(result, Err(GatewayError::InternalServerError(_))),
            "expected the database lookup to be reached, got {result:?}"
        );
    }

    /// Rewrite the raw beans of an encoded connection string.
    fn tamper(
        connection_string: &str,
        edit: impl Fn(&str) -> String,
    ) -> String {
        let raw = general_purpose::STANDARD.decode(connection_string).unwrap();
        let raw = String::from_utf8(raw).unwrap();

        general_purpose::STANDARD.encode(edit(&raw))
    }

    #[actix_web::test]
    async fn general_string_passes_the_scope_gate() {
        let life_cycle = life_cycle();
        let req = request_with_connection_string(
            &general_connection_string(&life_cycle).await,
            &life_cycle,
        );

        assert_reached_database_lookup(
            fetch_connection_string_from_request(req).await,
        );
    }

    #[actix_web::test]
    async fn notification_string_is_rejected_on_general_surfaces() {
        let life_cycle = life_cycle();
        let req = request_with_connection_string(
            &notification_connection_string(&life_cycle).await,
            &life_cycle,
        );

        assert_unauthorized_with_code(
            fetch_connection_string_from_request(req).await,
            NativeErrorCodes::MYC00033,
        );
    }

    #[actix_web::test]
    async fn notification_extractor_rejects_general_string() {
        let life_cycle = life_cycle();
        let req = request_with_connection_string(
            &general_connection_string(&life_cycle).await,
            &life_cycle,
        );

        assert_unauthorized_with_code(
            fetch_notification_connection_string_from_request(req).await,
            NativeErrorCodes::MYC00034,
        );
    }

    #[actix_web::test]
    async fn notification_extractor_accepts_notification_string() {
        let life_cycle = life_cycle();
        let req = request_with_connection_string(
            &notification_connection_string(&life_cycle).await,
            &life_cycle,
        );

        assert_reached_database_lookup(
            fetch_notification_connection_string_from_request(req).await,
        );
    }

    #[actix_web::test]
    async fn notification_string_with_stripped_scope_fails_signature() {
        let life_cycle = life_cycle();
        let stripped =
            tamper(&notification_connection_string(&life_cycle).await, |raw| {
                raw.split(';')
                    .filter(|bean| !bean.starts_with("scp="))
                    .collect::<Vec<_>>()
                    .join(";")
            });

        assert_unauthorized_with_code(
            fetch_connection_string_from_request(
                request_with_connection_string(&stripped, &life_cycle),
            )
            .await,
            NativeErrorCodes::MYC00032,
        );
    }

    #[actix_web::test]
    async fn general_string_with_injected_scope_fails_signature() {
        let life_cycle = life_cycle();
        let injected =
            tamper(&general_connection_string(&life_cycle).await, |raw| {
                format!("scp=notify:email;svc=service-01;{raw}")
            });

        assert_unauthorized_with_code(
            fetch_notification_connection_string_from_request(
                request_with_connection_string(&injected, &life_cycle),
            )
            .await,
            NativeErrorCodes::MYC00032,
        );
    }
}
