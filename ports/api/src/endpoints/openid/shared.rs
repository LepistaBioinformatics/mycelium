use actix_web::HttpResponse;
use myc_config::optional_config::OptionalConfig;
use myc_http_tools::models::{
    auth_config::AuthConfig, external_providers_config::ExternalProviderConfig,
};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, Clone, Deserialize, Serialize, ToResponse, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct AuthorizationProvider {
    pub(super) issuer: String,
    pub(super) discovery_url: String,
}

pub(super) async fn get_authorization_providers(
    auth_config: &AuthConfig,
    custom_external_config: Option<Vec<ExternalProviderConfig>>,
) -> Result<Vec<AuthorizationProvider>, HttpResponse> {
    let external_config = if let Some(config) = custom_external_config {
        config
    } else {
        if let OptionalConfig::Enabled(config) = &auth_config.external {
            config
        } else {
            return Err(HttpResponse::NotFound()
                .body("External providers are not configured"));
        }
        .to_vec()
    };

    let mut authorization_servers = vec![];

    for provider in external_config {
        let discovery_url =
            if let Some(discovery_url) = provider.discovery_url.to_owned() {
                match discovery_url.async_get_or_error().await {
                    Ok(url) => url,
                    Err(_) => {
                        return Err(HttpResponse::NotFound()
                            .body("Discovery URL is not configured"));
                    }
                }
            } else {
                return Err(HttpResponse::NotFound()
                    .body("Discovery URL is not configured"));
            };

        authorization_servers.push(AuthorizationProvider {
            issuer: provider.issuer.async_get_or_error().await.unwrap(),
            discovery_url,
        });
    }

    Ok(authorization_servers)
}
