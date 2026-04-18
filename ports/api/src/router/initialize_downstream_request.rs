use crate::models::api_config::ApiConfig;

use actix_web::{web, HttpRequest};
use awc::{Client, ClientRequest};
use myc_core::domain::dtos::route::Route;
use myc_http_tools::{
    responses::GatewayError,
    settings::{FORWARD_FOR_KEY, MYCELIUM_SERVICE_NAME, RFC7239_FORWARDED_KEY},
};
use mycelium_base::dtos::Parent;
use std::time::Duration;
use url::Url;

// ? ---------------------------------------------------------------------------
// ? IP resolution helpers (RFC 7239)
// ? ---------------------------------------------------------------------------

/// Extract the first client IP from a RFC 7239 `Forwarded` header value.
///
/// Handles the following `for=` token forms (RFC 7239 §6):
/// - Bare IPv4: `for=192.0.2.1`
/// - Quoted with port: `for="192.0.2.1:4711"`
/// - Quoted IPv6 with port: `for="[2001:db8::1]:4711"`
/// - Multiple directives separated by `,` (leftmost = closest to client)
/// - Parameters after `;` are ignored
///
pub(crate) fn parse_forwarded_for(value: &str) -> Option<String> {
    // Take the first comma-separated entry (closest to the client).
    let first_entry = value.split(',').next()?;

    // Find a `for=` token (case-insensitive) among `;`-separated params.
    let for_token = first_entry
        .split(';')
        .find(|part| part.trim().to_lowercase().starts_with("for="))?;

    let raw = for_token.trim().splitn(2, '=').nth(1)?.trim();

    // Strip surrounding quotes if present.
    let unquoted = raw.strip_prefix('"').and_then(|s| s.strip_suffix('"'));
    let candidate = unquoted.unwrap_or(raw);

    // IPv6 literal in brackets: `[2001:db8::1]` or `[2001:db8::1]:port`.
    if let Some(inner) = candidate.strip_prefix('[') {
        return inner.split(']').next().map(str::to_owned);
    }

    // Bare IPv4 or `192.0.2.1:port` — strip trailing port if present.
    let ip = candidate.splitn(2, ':').next()?;
    Some(ip.to_owned())
}

/// Resolve the original client IP using priority order:
/// 1. RFC 7239 `Forwarded: for=<ip>`
/// 2. `X-Forwarded-For` (first value)
/// 3. TCP `peer_addr`
///
fn resolve_client_ip(req: &HttpRequest) -> Option<String> {
    if let Some(forwarded) = req.headers().get(RFC7239_FORWARDED_KEY) {
        let value = forwarded.to_str().ok()?;
        let parsed = parse_forwarded_for(value);
        if parsed.is_some() {
            return parsed;
        }
    }

    if let Some(xff) = req.headers().get(FORWARD_FOR_KEY) {
        let value = xff.to_str().ok()?;
        let ip = value.split(',').next()?.trim().to_owned();
        if !ip.is_empty() {
            return Some(ip);
        }
    }

    req.head().peer_addr.map(|addr| addr.ip().to_string())
}

// ? ---------------------------------------------------------------------------
// ? Downstream request initialization
// ? ---------------------------------------------------------------------------

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
) -> Result<ClientRequest, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Extract service name from the route matching uri
    //
    // Extract the service name from the route matching uri. This is used to
    // adjust the downstream path to include the service name. Otherwise, the
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
    // ? Build URI from matching route
    //
    // Build the registered uri from the route. This uri is the uri that the
    // gateway will use to forward the request to the service. The URI can
    // include wildcards and variables.
    //
    // Example:
    //
    // ```
    // http://my-service:8083/public*
    // ```
    //
    // ? -----------------------------------------------------------------------

    let route_matching_uri = route.build_uri().await.map_err(|err| {
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

    let mut target_url = Url::parse(route_matching_uri.to_string().as_str())
        .map_err(|err| {
            tracing::warn!("{:?}", err);
            GatewayError::InternalServerError(format!("{err}"))
        })?;

    target_url.set_path(
        req.uri()
            .path()
            .replace(
                format!("/{name}", name = service.name.to_owned()).as_str(),
                "",
            )
            .as_str(),
    );

    target_url.set_query(req.uri().query());

    //
    // If the proxy address exists, the downstream url should be adjusted to
    // concatenate the proxy address with the service url.
    //
    // Example:
    //
    // ```
    // # Original url
    // http://my-service:8083/public?test=value
    //
    // # With proxy address
    // http://localhost:8888/http://my-service:8083/public?test=value
    // ```
    //
    let routing_url =
        if let Some(proxy_address) = service.proxy_address.to_owned() {
            let proxy_url = format!(
                "{}/{}",
                proxy_address.as_str(),
                target_url.to_owned().to_string().as_str()
            );

            Url::parse(proxy_url.as_str()).map_err(|err| {
                tracing::warn!("{:?}", err);
                GatewayError::InternalServerError(format!("{err}"))
            })?
        } else {
            target_url.to_owned()
        };

    // ? -----------------------------------------------------------------------
    // ? Resolve the original client IP and build forwarding headers
    //
    // Priority order: RFC 7239 `Forwarded: for=<ip>` → `X-Forwarded-For`
    // → TCP peer_addr. Both the legacy `X-Forwarded-For` and the standard
    // RFC 7239 `Forwarded` headers are injected into the downstream request.
    //
    // ? -----------------------------------------------------------------------

    let client_ip = resolve_client_ip(&req).unwrap_or_default();

    let mut downstream_request = client
        .request_from(routing_url.as_str(), req.head())
        .no_decompress()
        .timeout(Duration::from_secs(config.gateway_timeout))
        .insert_header((FORWARD_FOR_KEY, client_ip.as_str()))
        .insert_header((RFC7239_FORWARDED_KEY, format!("for={client_ip}")));

    //
    // Inform to the downstream service about the target host, protocol and
    // port. Also, inform that the request is coming from the mycelium gateway.
    //
    downstream_request = downstream_request
        .insert_header((MYCELIUM_SERVICE_NAME, format!("{}", service.name)));

    Ok(downstream_request)
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::parse_forwarded_for;

    #[test]
    fn bare_ipv4() {
        let result = parse_forwarded_for("for=192.0.2.60");
        assert_eq!(result, Some("192.0.2.60".to_owned()));
    }

    #[test]
    fn quoted_ipv4_with_port() {
        let result = parse_forwarded_for("for=\"192.0.2.43:47011\"");
        assert_eq!(result, Some("192.0.2.43".to_owned()));
    }

    #[test]
    fn quoted_ipv6_with_port() {
        let result = parse_forwarded_for("for=\"[2001:db8::1]:4711\"");
        assert_eq!(result, Some("2001:db8::1".to_owned()));
    }

    #[test]
    fn multiple_values_takes_leftmost() {
        let result = parse_forwarded_for("for=192.0.2.60, for=198.51.100.17");
        assert_eq!(result, Some("192.0.2.60".to_owned()));
    }

    #[test]
    fn semicolon_separated_params_ignored() {
        let result =
            parse_forwarded_for("for=192.0.2.60;proto=http;by=10.0.0.1");
        assert_eq!(result, Some("192.0.2.60".to_owned()));
    }

    #[test]
    fn case_insensitive_for_key() {
        let result = parse_forwarded_for("For=192.0.2.60");
        assert_eq!(result, Some("192.0.2.60".to_owned()));
    }

    #[test]
    fn missing_for_key_returns_none() {
        let result = parse_forwarded_for("proto=https;by=10.0.0.1");
        assert_eq!(result, None);
    }

    #[test]
    fn empty_string_returns_none() {
        let result = parse_forwarded_for("");
        assert_eq!(result, None);
    }
}
