use crate::models::api_config::ApiConfig;

use actix_web::{web, HttpRequest};
use awc::{Client, ClientRequest};
use myc_core::domain::dtos::route::Route;
use myc_http_tools::{responses::GatewayError, settings::FORWARD_FOR_KEY};
use mycelium_base::dtos::Parent;
use std::time::Duration;
use url::Url;

/// Initialize the downstream request
///
/// This function initializes the downstream request by checking the protocol
/// permission and the source reliability.
///
#[tracing::instrument(name = "initialize_downstream_request", skip_all)]
pub(super) async fn initialize_downstream_request(
    req: HttpRequest,
    route: &Route,
    client: web::Data<Client>,
    config: web::Data<ApiConfig>,
    gateway_base_path: &str,
) -> Result<ClientRequest, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Build the registered uri from the route
    //
    // Build the registered uri from the route. This uri is the uri that the
    // gateway will use to forward the request to the service.
    //
    // ? -----------------------------------------------------------------------

    let uri = route.build_uri().await.map_err(|err| {
        tracing::warn!("{:?}", err);
        GatewayError::InternalServerError(format!("{err}"))
    })?;

    // ? -----------------------------------------------------------------------
    // ? Parse the registered uri as a url
    //
    // Parse the registered uri as a url. This url is the url that the gateway
    // will use to forward the request to the service.
    //
    // ? -----------------------------------------------------------------------

    let mut url = Url::parse(uri.to_string().as_str()).map_err(|err| {
        tracing::warn!("{:?}", err);
        GatewayError::InternalServerError(format!("{err}"))
    })?;

    // ? -----------------------------------------------------------------------
    // ? Adjust downstream path
    //
    // Adjust the downstream path to include the service name. Otherwise, the
    // downstream request will not be able to find the service. Also remote the
    // gateway base path from the request path.
    //
    // ? -----------------------------------------------------------------------

    let service = match route.service {
        Parent::Record(ref service) => service,
        Parent::Id(_) => {
            tracing::error!("Service not found");

            return Err(GatewayError::InternalServerError(String::from(
                "Service not found",
            )));
        }
    };

    url.set_path(
        req.uri()
            .path()
            .replace(gateway_base_path, "")
            .replace(
                format!("/{name}", name = service.name.to_owned()).as_str(),
                "",
            )
            .as_str(),
    );

    url.set_query(req.uri().query());

    // ? -----------------------------------------------------------------------
    // ? Build the downstream request
    //
    // Build the downstream request by setting the forward for key. Request
    // configs like timeout and decompression are also set. It should prevent
    // the request to be consumed by the gateway before the downstream request
    // is sent.
    //
    // ? -----------------------------------------------------------------------

    let forward_for_key = if let Some(addr) = req.head().peer_addr {
        Some(format!("{}", addr.ip()))
    } else {
        None
    };

    let downstream_request = client
        .request_from(url.as_str(), req.head())
        .no_decompress()
        .timeout(Duration::from_secs(config.gateway_timeout))
        .insert_header((FORWARD_FOR_KEY, forward_for_key.unwrap_or_default()));

    Ok(downstream_request)
}
