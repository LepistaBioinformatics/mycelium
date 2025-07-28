use crate::endpoints::openid::shared::{
    get_authorization_providers, AuthorizationProvider,
};

use actix_web::{get, web, HttpResponse, Responder};
use myc_config::optional_config::OptionalConfig;
use myc_core::models::AccountLifeCycle;
use myc_http_tools::{
    models::auth_config::AuthConfig, settings::DEFAULT_CONNECTION_STRING_KEY,
};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(well_known_oauth_authorization_server)
        .service(well_known_protected_resource);
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize, ToResponse, ToSchema)]
#[serde(rename_all = "camelCase")]
struct ProtectedResource {
    resource: String,
    authorization_servers: Vec<AuthorizationProvider>,
    scopes_supported: Vec<String>,
    bearer_methods_supported: Vec<String>,
    resource_documentation: String,
}

/// Provide the well known openid configuration endpoint.
///
/// This endpoint is used to get the well known openid configuration from the
/// auth0 server.
///
#[utoipa::path(
        get,
        responses(
            (
                status = 200,
                description = "Well known oauth authorization server.",
                body = AuthorizationProvider,
            ),
        ),
    )
]
#[get("/.well-known/oauth-authorization-server")]
pub async fn well_known_oauth_authorization_server(
    auth_config: web::Data<AuthConfig>,
) -> impl Responder {
    let auth_config = auth_config.get_ref();

    let external_config =
        if let OptionalConfig::Enabled(config) = &auth_config.external {
            config
        } else {
            return HttpResponse::NotFound()
                .body("External providers are not configured");
        };

    let eligible_providers = external_config
        .iter()
        .filter(|provider| provider.discovery_url.is_some())
        .map(|provider| provider.clone())
        .collect::<Vec<_>>();

    let authorization_providers = match get_authorization_providers(
        auth_config,
        Some(eligible_providers),
    )
    .await
    {
        Ok(providers) => providers,
        Err(error) => {
            return error;
        }
    };

    if authorization_providers.is_empty() {
        return HttpResponse::NotFound()
            .body("No authorization providers are configured");
    }

    let provider = authorization_providers.iter().next().unwrap();

    HttpResponse::Found()
        .append_header(("Location", provider.discovery_url.clone()))
        .finish()
}

/// Provide the well known auth protected resources endpoint
///
/// This endpoint is used to get the well known auth protected resources from
/// the auth0 server.
///
/// Example:
///
/// ```json
/// {
///     "resource": "https://api.mycelium.example.com",
///     "authorization_servers": [
///         {
///             "issuer": "https://auth0.example.com",
///             "metadata": "https://auth0.example.com/.well-known/openid-configuration"
///         },
///         {
///             "issuer": "https://accounts.google.com",
///             "metadata": "https://accounts.google.com/.well-known/openid-configuration"
///         }
///     ],
///     "scopes_supported": ["read", "write"],
///     "bearer_methods_supported": ["header", "x-mycelium-connection-string"],
///     "resource_documentation": "https://lepistabioinformatics.github.io/mycelium-docs/"
/// }
/// ```
///
#[utoipa::path(
        get,
        responses(
            (
                status = 200,
                description = "Well known oauth protected resource.",
                body = AuthorizationProvider,
            ),
        ),
    )
]
#[get("/.well-known/oauth-protected-resource")]
pub async fn well_known_protected_resource(
    auth_config: web::Data<AuthConfig>,
    account_life_cycle: web::Data<AccountLifeCycle>,
) -> impl Responder {
    let auth_config = auth_config.get_ref();

    let authorization_servers =
        match get_authorization_providers(auth_config, None).await {
            Ok(providers) => providers,
            Err(error) => {
                return error;
            }
        };

    let resource = if let Some(domain_url) =
        account_life_cycle.domain_url.clone()
    {
        domain_url.async_get_or_error().await.unwrap()
    } else {
        return HttpResponse::NotFound().body("Domain URL is not configured");
    };

    let protected_resource = ProtectedResource {
        resource,
        authorization_servers,
        scopes_supported: vec![],
        bearer_methods_supported: vec![
            "header".to_string(),
            DEFAULT_CONNECTION_STRING_KEY.to_string(),
        ],
        resource_documentation:
            "https://lepistabioinformatics.github.io/mycelium-docs/".to_string(),
    };

    HttpResponse::Ok().json(protected_resource)
}
