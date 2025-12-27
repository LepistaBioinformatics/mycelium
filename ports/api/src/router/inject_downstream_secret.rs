use crate::models::api_config::ApiConfig;

use actix_web::web;
use awc::ClientRequest;
use myc_config::optional_config::OptionalConfig;
use myc_core::domain::dtos::{
    http::Protocol, http_secret::HttpSecret, route::Route,
};
use myc_http_tools::responses::GatewayError;
use mycelium_base::dtos::Parent;

/// Inject the downstream secret into the request
///
/// This function injects the downstream secret into the request if the service
/// requested by the route has a secret.
///
#[tracing::instrument(name = "inject_downstream_secret", skip_all)]
pub(super) async fn inject_downstream_secret(
    mut req: ClientRequest,
    route: Route,
    mut route_key: Option<String>,
    api_config: web::Data<ApiConfig>,
) -> Result<(ClientRequest, Option<String>), GatewayError> {
    let _ = tracing::Span::current();

    let route_secret = match route.solve_secret().await {
        Err(err) => {
            tracing::warn!("{:?}", err);
            return Err(GatewayError::InternalServerError(format!("{err}")));
        }
        Ok(res) => res,
    };

    if let Some(secret) = route_secret {
        let accept_insecure_routing =
            route.accept_insecure_routing.unwrap_or(false);

        //
        // Check if the service supports TLS
        //
        if let OptionalConfig::Disabled = api_config.tls {
            if !accept_insecure_routing {
                tracing::error!("Secrets are only allowed for HTTPS routes");

                return Err(GatewayError::InternalServerError(
                    "Unexpected error on route request".to_string(),
                ));
            }
        }

        //
        // Check if the route supports HTTPS
        //
        if ![Protocol::Https].contains(&match route.service {
            Parent::Record(ref service) => service.protocol,
            Parent::Id(_) => {
                tracing::error!("Service not found");

                return Err(GatewayError::InternalServerError(String::from(
                    "Service not found",
                )));
            }
        }) {
            if !accept_insecure_routing {
                tracing::error!(
                    "Secrets are only allowed for HTTPS routes: {path}",
                    path = route.path
                );

                return Err(GatewayError::InternalServerError(
                    "Unexpected error on route request".to_string(),
                ));
            }
        }

        match secret {
            //
            // Insert the authorization key into the header
            //
            HttpSecret::AuthorizationHeader {
                header_name,
                prefix,
                token,
            } => {
                //
                // Build the bearer token
                //
                let mut bearer_token = prefix.unwrap_or("Bearer".to_string());
                bearer_token.push_str(format!(" {}", token).as_str());
                let bearer_name =
                    header_name.unwrap_or("Authorization".to_string());
                route_key = Some(bearer_name.to_owned());

                //
                // Remove any previous Authorization header that may exist
                //
                req.headers_mut().remove(bearer_name.to_owned());
                req.headers_mut()
                    .remove(bearer_name.to_lowercase().to_owned());
                req.headers_mut()
                    .remove(bearer_name.to_uppercase().to_owned());

                //
                // Insert the new Authorization header
                //
                req = req.insert_header((bearer_name, bearer_token));
            }
            //
            // Insert the query parameter into the header
            //
            HttpSecret::QueryParameter { name, token } => {
                req = match req.query(&[(name, token.to_owned())]) {
                    Err(err) => {
                        tracing::warn!("{:?}", err);

                        return Err(GatewayError::InternalServerError(
                            format!("{err}"),
                        ));
                    }
                    Ok(res) => res,
                };
            }
        }
    };

    Ok((req, route_key))
}
