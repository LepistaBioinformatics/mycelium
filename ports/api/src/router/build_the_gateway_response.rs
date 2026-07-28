use actix_web::{
    http::{header::HeaderMap, StatusCode},
    HttpResponse, HttpResponseBuilder,
};
use awc::error::HeaderValue;
use myc_http_tools::{
    responses::GatewayError,
    settings::{
        DEFAULT_REQUEST_ID_KEY, FORWARDING_KEYS, FORWARD_FOR_KEY,
        MYCELIUM_HEADER_PREFIX, RFC7239_FORWARDED_KEY,
    },
};

/// Build the gateway response
///
/// This function builds the gateway response with the downstream response.
///
#[tracing::instrument(
    name = "build_the_gateway_response",
    skip_all,
    fields(
        myc.router.res_size = tracing::field::Empty,
    )
)]
pub(super) async fn build_the_gateway_response(
    request_id: Option<HeaderValue>,
    route_key: Option<String>,
    downstream_status: StatusCode,
    downstream_headers: &HeaderMap,
) -> Result<HttpResponseBuilder, GatewayError> {
    let span = tracing::Span::current();

    let mut gateway_response = HttpResponse::build(downstream_status);

    if let Some(request_id) = request_id {
        gateway_response
            .insert_header((DEFAULT_REQUEST_ID_KEY, request_id.to_owned()));
    }

    let blocked_headers = blocked_response_headers(route_key);

    //
    // Forward the headers of the response before send it to the client
    //
    // `append_header` instead of `insert_header`: `HeaderMap::iter` yields one
    // entry per value, so multi-valued headers such as `Set-Cookie` would be
    // collapsed to their last value by an insert.
    //
    for (header_name, header_value) in
        downstream_headers.iter().filter(|(name, _)| {
            !is_blocked_response_header(name.as_str(), &blocked_headers)
        })
    {
        gateway_response
            .append_header((header_name.clone(), header_value.clone()));
    }

    if let Some(size) = downstream_headers
        .get("content-length")
        .map(|h| h.to_str().unwrap_or("0").parse::<u64>().unwrap_or(0))
    {
        // Add the response size to the metric
        span.record("myc.router.res_size", &Some(size));
    }

    // Add the downstream response status to the metric
    span.record("myc.router.res_status", &Some(downstream_status.as_u16()));

    Ok(gateway_response)
}

/// Collect the response headers that must not be forwarded to the client
///
/// ! Blocked response headers
///
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
///
/// Two distinct sets are removed from the downstream response before it is
/// streamed back to the client:
///
/// 1. Hop-by-hop headers (`FORWARDING_KEYS`), which RFC 7230 § 6.1 forbids a
///    proxy from forwarding.
///
/// 2. Gateway-injected artifacts, which travel in the request direction only.
///    If a downstream service echoes one of them back, forwarding it would leak
///    internal context to the client — `route_key` in particular is the name of
///    the injected downstream secret header, and the `x-mycelium-` namespace
///    carries identity and authorization context (profile, email, security
///    group, connection string).
///
/// Every other header is forwarded verbatim. Both sets contain sensitive
/// information about the system internals. Thus, be careful on edit this
/// section.
///
/// Names are lowercased on both sides of the comparison, as HTTP header names
/// are case-insensitive.
///
fn blocked_response_headers(route_key: Option<String>) -> Vec<String> {
    let mut blocked_headers = FORWARDING_KEYS
        .iter()
        .map(|key| key.to_lowercase())
        .collect::<Vec<String>>();

    blocked_headers.append(&mut vec![
        FORWARD_FOR_KEY.to_lowercase(),
        RFC7239_FORWARDED_KEY.to_lowercase(),
    ]);

    let Some(key) = route_key else {
        return blocked_headers;
    };

    blocked_headers.push(key.to_lowercase());
    blocked_headers
}

/// Whether a downstream response header must not reach the client
///
/// The whole `x-mycelium-` namespace is matched by prefix rather than by
/// listing the keys. Enumerating them is what let `x-mycelium-email`,
/// `x-mycelium-security-group` and `x-mycelium-connection-string` through: a
/// list only covers the keys that existed when it was written.
///
fn is_blocked_response_header(
    header_name: &str,
    blocked_headers: &[String],
) -> bool {
    if header_name.starts_with(MYCELIUM_HEADER_PREFIX) {
        return true;
    }

    blocked_headers.contains(&header_name.to_owned())
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::build_the_gateway_response;

    use actix_web::http::{
        header::{HeaderMap, HeaderName, HeaderValue},
        StatusCode,
    };
    use awc::error::HeaderValue as AwcHeaderValue;
    use myc_http_tools::settings::{
        DEFAULT_CONNECTION_STRING_KEY, DEFAULT_EMAIL_KEY,
        DEFAULT_MYCELIUM_ROLE_KEY, DEFAULT_PROFILE_KEY, DEFAULT_REQUEST_ID_KEY,
        DEFAULT_SCOPE_KEY, DEFAULT_TENANT_ID_KEY, MYCELIUM_SECURITY_GROUP,
        MYCELIUM_SERVICE_NAME,
    };

    fn headers_from(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();

        for (name, value) in pairs {
            headers.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        headers
    }

    #[tokio::test]
    async fn streaming_content_type_reaches_the_client() {
        let downstream_headers =
            headers_from(&[("content-type", "text/event-stream")]);

        let mut builder = build_the_gateway_response(
            None,
            None,
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/event-stream"
        );
    }

    #[tokio::test]
    async fn hop_by_hop_headers_are_removed() {
        let downstream_headers = headers_from(&[
            ("connection", "keep-alive"),
            ("transfer-encoding", "chunked"),
            ("upgrade", "websocket"),
        ]);

        let mut builder = build_the_gateway_response(
            None,
            None,
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        assert!(response.headers().get("connection").is_none());
        assert!(response.headers().get("transfer-encoding").is_none());
        assert!(response.headers().get("upgrade").is_none());
    }

    #[tokio::test]
    async fn application_headers_are_forwarded() {
        let downstream_headers = headers_from(&[
            ("x-crab-shell-session", "session-42"),
            ("cache-control", "no-cache"),
            ("etag", "\"abc123\""),
            ("content-encoding", "gzip"),
        ]);

        let mut builder = build_the_gateway_response(
            None,
            None,
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        assert_eq!(
            response.headers().get("x-crab-shell-session").unwrap(),
            "session-42"
        );

        assert_eq!(
            response.headers().get("cache-control").unwrap(),
            "no-cache"
        );
        assert_eq!(response.headers().get("etag").unwrap(), "\"abc123\"");
        assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");
    }

    #[tokio::test]
    async fn echoed_gateway_artifacts_are_stripped() {
        let downstream_headers = headers_from(&[
            ("x-downstream-secret", "super-secret-token"),
            (DEFAULT_PROFILE_KEY, "leaked-profile"),
            (MYCELIUM_SERVICE_NAME, "crab-shell-proxy"),
            ("x-forwarded-for", "10.0.0.1"),
        ]);

        let mut builder = build_the_gateway_response(
            None,
            Some("x-downstream-secret".to_owned()),
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        assert!(response.headers().get("x-downstream-secret").is_none());
        assert!(response.headers().get(DEFAULT_PROFILE_KEY).is_none());
        assert!(response.headers().get(MYCELIUM_SERVICE_NAME).is_none());
        assert!(response.headers().get("x-forwarded-for").is_none());
    }

    #[tokio::test]
    async fn echoed_request_id_does_not_override_the_gateway_one() {
        let downstream_headers =
            headers_from(&[(DEFAULT_REQUEST_ID_KEY, "downstream-echo")]);

        let mut builder = build_the_gateway_response(
            Some(AwcHeaderValue::from_static("gateway-request-id")),
            None,
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        assert_eq!(
            response.headers().get(DEFAULT_REQUEST_ID_KEY).unwrap(),
            "gateway-request-id"
        );
    }

    #[tokio::test]
    async fn the_whole_mycelium_namespace_is_stripped() {
        //
        // Anything the gateway may have injected into the request leaks
        // internal context if a downstream echoes it back. The namespace is
        // matched by prefix, so keys added later are covered too.
        //
        let downstream_headers = headers_from(&[
            (DEFAULT_EMAIL_KEY, "user@example.com"),
            (MYCELIUM_SECURITY_GROUP, "{\"Protected\":null}"),
            (DEFAULT_CONNECTION_STRING_KEY, "user-credential"),
            (DEFAULT_SCOPE_KEY, "leaked-scope"),
            (DEFAULT_MYCELIUM_ROLE_KEY, "leaked-role"),
            (DEFAULT_TENANT_ID_KEY, "leaked-tenant"),
            ("x-mycelium-not-yet-invented", "future-key"),
        ]);

        let mut builder = build_the_gateway_response(
            None,
            None,
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        assert!(response.headers().get(DEFAULT_EMAIL_KEY).is_none());
        assert!(response.headers().get(MYCELIUM_SECURITY_GROUP).is_none());
        assert!(response
            .headers()
            .get(DEFAULT_CONNECTION_STRING_KEY)
            .is_none());
        assert!(response.headers().get(DEFAULT_SCOPE_KEY).is_none());
        assert!(response.headers().get(DEFAULT_MYCELIUM_ROLE_KEY).is_none());
        assert!(response.headers().get(DEFAULT_TENANT_ID_KEY).is_none());

        assert!(response
            .headers()
            .get("x-mycelium-not-yet-invented")
            .is_none());
    }

    #[tokio::test]
    async fn multi_valued_headers_keep_every_value() {
        let downstream_headers = headers_from(&[
            ("set-cookie", "session=abc"),
            ("set-cookie", "csrf=xyz"),
        ]);

        let mut builder = build_the_gateway_response(
            None,
            None,
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        let cookies = response
            .headers()
            .get_all("set-cookie")
            .map(|value| value.to_str().unwrap().to_owned())
            .collect::<Vec<String>>();

        assert_eq!(cookies, vec!["session=abc", "csrf=xyz"]);
    }

    #[tokio::test]
    async fn blocklist_matching_is_case_insensitive() {
        //
        // `FORWARDING_KEYS` are capitalized and the route key comes from the
        // route configuration with arbitrary casing, while actix normalizes
        // header names to lowercase.
        //
        let downstream_headers = headers_from(&[
            ("Connection", "keep-alive"),
            ("X-Downstream-Secret", "super-secret-token"),
            ("Content-Type", "application/json"),
        ]);

        let mut builder = build_the_gateway_response(
            None,
            Some("X-Downstream-Secret".to_owned()),
            StatusCode::OK,
            &downstream_headers,
        )
        .await
        .unwrap();

        let response = builder.finish();

        assert!(response.headers().get("connection").is_none());
        assert!(response.headers().get("x-downstream-secret").is_none());

        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );
    }
}
