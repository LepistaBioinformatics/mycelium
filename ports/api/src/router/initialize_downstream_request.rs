use crate::models::api_config::ApiConfig;

use actix_web::{web, HttpRequest};
use awc::{Client, ClientRequest};
use myc_core::domain::dtos::route::Route;
use myc_http_tools::{
    responses::GatewayError,
    settings::{
        FORWARD_FOR_KEY, MYCELIUM_ROUTING_KEY, MYCELIUM_TARGET_HOST,
        MYCELIUM_TARGET_PORT, MYCELIUM_TARGET_PROTOCOL,
    },
};
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

    // ? -----------------------------------------------------------------------
    // ? Parse the registered uri as a url
    //
    // Parse the registered uri as a url. This url is the url that the gateway
    // will use to forward the request to the service.
    //
    // ? -----------------------------------------------------------------------

    let raw_url = Url::parse(uri.to_string().as_str()).map_err(|err| {
        tracing::warn!("{:?}", err);
        GatewayError::InternalServerError(format!("{err}"))
    })?;

    //
    // If the proxy address exists, the downstream url should be adjusted to
    // concatenate the proxy address with the service url.
    //
    // Example:
    //
    // ```
    // # Original url
    // http://service.example.com:8080/api/v1/service/1234567890
    //
    // # With proxy address
    // http://proxy.example.com:8080/api/v1/service/1234567890
    // ```
    //
    let mut url = if let Some(proxy_address) = service.proxy_address.to_owned()
    {
        let proxy_url = format!(
            "{}/{}",
            proxy_address.as_str(),
            raw_url.to_owned().to_string().as_str()
        );

        Url::parse(proxy_url.as_str()).map_err(|err| {
            tracing::warn!("{:?}", err);
            GatewayError::InternalServerError(format!("{err}"))
        })?
    } else {
        raw_url.to_owned()
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

    let mut downstream_request = client
        .request_from(url.as_str(), req.head())
        .no_decompress()
        .timeout(Duration::from_secs(config.gateway_timeout))
        .insert_header((FORWARD_FOR_KEY, forward_for_key.unwrap_or_default()));

    //
    // Inform to the downstream service about the target host, protocol and
    // port. Also, inform that the request is coming from the mycelium gateway.
    //
    if let Some(_) = service.proxy_address.to_owned() {
        downstream_request = downstream_request
            .insert_header((
                MYCELIUM_TARGET_HOST,
                format!("{}", raw_url.host_str().unwrap_or_default()),
            ))
            .insert_header((
                MYCELIUM_TARGET_PROTOCOL,
                raw_url.scheme().to_string(),
            ))
            .insert_header((
                MYCELIUM_TARGET_PORT,
                raw_url.port().unwrap_or(80).to_string(),
            ))
            .insert_header((MYCELIUM_ROUTING_KEY, "true"));
    };

    Ok(downstream_request)
}
