use std::collections::HashMap;
use std::time::Instant;

use actix_web::{
    dev::{Decompress, Payload as DevPayload},
    web::Payload,
    HttpRequest,
};
use awc::{
    error::{ConnectError, SendRequestError},
    ClientRequest, ClientResponse,
};
use chrono::Utc;
use myc_core::domain::dtos::{
    callback::{CallbackContext, CallbackManager, UserInfo},
    http::HttpMethod,
};
use myc_http_tools::{
    responses::GatewayError, settings::DEFAULT_REQUEST_ID_KEY, SecurityGroup,
};
use myc_mem_db::{
    models::config::DbPoolProvider, repositories::MemDbAppModule,
};
use shaku::HasComponent;

pub(super) type DownstreamResponse = ClientResponse<Decompress<DevPayload>>;

/// Stream the request to the downstream service
///
/// This function streams the request to the downstream service.
///
/// Returns the binding response and the client response. Downstream response
/// contains the route response body and important headers, where the client
/// response contains the gateway response headers.
///
#[tracing::instrument(name = "stream_request_to_downstream", skip_all)]
pub(super) async fn stream_request_to_downstream(
    downstream_request: ClientRequest,
    upstream_request: &HttpRequest,
    payload: Payload,
    callback_names: Option<Vec<String>>,
    mem_module: &MemDbAppModule,
    user_info: Option<UserInfo>,
    security_group: SecurityGroup,
) -> Result<DownstreamResponse, GatewayError> {
    let _ = tracing::Span::current();

    // Capture start time for duration calculation
    let start_time = Instant::now();

    // Extract downstream metadata from the request (before it's moved)
    let (downstream_url, downstream_method) =
        get_downstream_request_metadata(&downstream_request)?;

    let mut downstream_response_headers = HashMap::<String, String>::new();
    for (name, value) in downstream_request.headers() {
        if let Ok(value_str) = value.to_str() {
            downstream_response_headers
                .insert(name.as_str().to_string(), value_str.to_string());
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Get engines and execution mode from memory db
    //
    // Retrieve the pre-initialized engines and execution mode from the
    // MemDbPoolProvider. If callback names are specified, filter engines to
    // only include those matching the callback names.
    //
    // ? -----------------------------------------------------------------------
    let db_provider: &dyn DbPoolProvider = mem_module.resolve_ref();
    let execution_mode = db_provider.get_execution_mode();

    let engines_to_execute = if let Some(names) = callback_names {
        db_provider.get_engines_by_names(&names)
    } else {
        db_provider.get_engines()
    };

    // Get callbacks for filter checking
    let callbacks = db_provider.get_callbacks_db();

    let downstream_response: DownstreamResponse = match downstream_request
        .send_stream(payload)
        .await
    {
        Err(err) => match err {
            SendRequestError::Connect(e) => {
                match e {
                    ConnectError::SslIsNotSupported => {
                        tracing::error!("SSL is not supported");

                        return Err(GatewayError::InternalServerError(
                            "SSL is not supported".to_string(),
                        ));
                    }
                    ConnectError::SslError(e) => {
                        tracing::error!("SSL error: {e}");

                        return Err(GatewayError::InternalServerError(
                            "SSL error".to_string(),
                        ));
                    }
                    ConnectError::Io(e) => {
                        tracing::error!("IO error: {e}");

                        return Err(GatewayError::BadGateway(
                            "Service temporarily unavailable".to_string(),
                        ));
                    }
                    _ => (),
                }

                tracing::warn!("Error on route/connect to service: {e}");

                return Err(GatewayError::InternalServerError(format!(
                    "Unexpected error on route request: {}",
                    e.to_string()
                )));
            }
            SendRequestError::Url(e) => {
                tracing::error!("Error on route/url to service: {e}");

                return Err(GatewayError::InternalServerError(String::from(
                    format!("{e}"),
                )));
            }
            err => {
                tracing::error!("Error on route/stream to service: {err}");

                return Err(GatewayError::InternalServerError(String::from(
                    format!("{err}"),
                )));
            }
        },
        Ok(res) => {
            let status = res.status();
            let duration_ms = start_time.elapsed().as_millis() as u64;

            // Extract HTTP metadata from upstream request
            // (downstream method already extracted before request was sent)
            let (upstream_path, request_id, client_ip) =
                get_upstream_request_metadata(upstream_request)?;

            let http_method = downstream_method;

            // Extract headers from downstream response
            for (name, value) in res.headers() {
                if let Ok(value_str) = value.to_str() {
                    downstream_response_headers.insert(
                        name.as_str().to_string(),
                        value_str.to_string(),
                    );
                }
            }
            //
            // Filter engines based on callback triggering filters
            //
            // Apply filters (triggering_methods, triggering_status_codes,
            // triggering_headers) to determine which callbacks should be
            // executed.
            //
            let mut callback_manager = CallbackManager::new(execution_mode);
            let status_code = status.as_u16();

            // Create a mapping of callback name to callback for filter checking
            let callback_map: HashMap<
                String,
                &myc_core::domain::dtos::callback::Callback,
            > = callbacks.iter().map(|cb| (cb.name.clone(), cb)).collect();

            // Create a mapping of engine name to callback name
            let engine_to_callback_map: HashMap<String, String> = callbacks
                .iter()
                .enumerate()
                .filter_map(|(idx, cb)| {
                    if idx < engines_to_execute.len() {
                        Some((
                            engines_to_execute[idx].name().to_string(),
                            cb.name.clone(),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            // Filter and register engines based on callback filters
            for engine in engines_to_execute {
                let engine_name = engine.name();
                if let Some(callback_name) =
                    engine_to_callback_map.get(engine_name)
                {
                    if let Some(callback) = callback_map.get(callback_name) {
                        // Check if callback should be executed based on filters
                        match callback.should_execute(
                            &http_method,
                            status_code,
                            &downstream_response_headers,
                        ) {
                            Ok(()) => {
                                callback_manager.register(engine);
                            }
                            Err(block_reason) => {
                                tracing::debug!(
                                    "Callback '{}' filtered out by {} (method: {:?}, status: {}, headers: {:?})",
                                    callback_name,
                                    block_reason,
                                    http_method,
                                    status_code,
                                    downstream_response_headers
                                );
                            }
                        }
                    } else {
                        // If callback not found, register engine anyway
                        // (backward compatibility)
                        callback_manager.register(engine);
                    }
                } else {
                    // If engine not mapped, register it anyway (backward
                    // compatibility)
                    callback_manager.register(engine);
                }
            }

            //
            // Execute callbacks
            //
            // Execute all registered callbacks with the response context.
            //
            callback_manager
                .execute_all(&CallbackContext::new(
                    status_code,
                    downstream_response_headers,
                    duration_ms,
                    upstream_path,
                    downstream_url.clone(),
                    http_method,
                    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    request_id,
                    client_ip,
                    user_info,
                    security_group,
                ))
                .await;

            if status.is_success() {
                tracing::trace!(
                    "Downstream response successfully received with status: {status}",
                    status = status,
                );
            } else {
                tracing::warn!(
                    "Downstream response received with status: {status}",
                    status = status
                );
            }

            res
        }
    };

    Ok(downstream_response)
}

/// Extract HTTP metadata from downstream request
///
/// Returns a tuple containing:
/// - downstream_url: The URL from the downstream request
/// - http_method: The HTTP method from the downstream request
fn get_downstream_request_metadata(
    downstream_request: &ClientRequest,
) -> Result<(String, HttpMethod), GatewayError> {
    // Extract downstream URL from the request
    let mut downstream_url = downstream_request.get_uri().to_string();
    // Remove trailing slash if present (normalize URL)
    if downstream_url.ends_with('/') && downstream_url.len() > 1 {
        downstream_url.pop();
    }

    // Extract HTTP method from the downstream request
    let http_method = match downstream_request.get_method().as_str() {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "DELETE" => HttpMethod::Delete,
        "PATCH" => HttpMethod::Patch,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        "CONNECT" => HttpMethod::Connect,
        "TRACE" => HttpMethod::Trace,
        _ => {
            return Err(GatewayError::InternalServerError(String::from(
                "Invalid HTTP method",
            )));
        }
    };

    Ok((downstream_url, http_method))
}

/// Extract HTTP metadata from upstream request
///
/// Returns a tuple containing:
/// - upstream_path: The path from the upstream request
/// - request_id: The request ID from upstream request headers (if available)
/// - client_ip: The client IP address from upstream request (if available)
fn get_upstream_request_metadata(
    upstream_request: &HttpRequest,
) -> Result<(String, Option<String>, Option<String>), GatewayError> {
    // Extract upstream path from the upstream request
    let upstream_path = upstream_request.uri().path().to_string();

    // Extract request ID from the upstream request headers
    let request_id = upstream_request
        .headers()
        .get(DEFAULT_REQUEST_ID_KEY)
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_string());

    // Extract client IP from the upstream request
    let client_ip = upstream_request
        .head()
        .peer_addr
        .map(|addr| addr.ip().to_string());

    Ok((upstream_path, request_id, client_ip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::Method, test::TestRequest};
    use awc::Client;

    // Helper function to create a ClientRequest for testing
    // Note: This creates a ClientRequest using request_from which returns
    // a builder that implements the same interface as ClientRequest
    fn create_test_client_request(method: Method, url: &str) -> ClientRequest {
        let client = Client::default();
        // Use request_from with a minimal HttpRequest to create a ClientRequest
        let test_req = TestRequest::default()
            .method(method.clone())
            .uri(url)
            .to_http_request();

        client.request_from(url, test_req.head())
    }

    #[actix_web::test]
    async fn test_get_downstream_request_metadata_success() {
        let request = create_test_client_request(
            Method::POST,
            "http://example.com/api/test",
        );

        let result = get_downstream_request_metadata(&request);

        assert!(result.is_ok());
        let (url, method) = result.unwrap();
        assert_eq!(url, "http://example.com/api/test");
        assert_eq!(method, HttpMethod::Post);
    }

    #[actix_web::test]
    async fn test_get_downstream_request_metadata_all_methods() {
        let methods = vec![
            (Method::GET, HttpMethod::Get),
            (Method::POST, HttpMethod::Post),
            (Method::PUT, HttpMethod::Put),
            (Method::DELETE, HttpMethod::Delete),
            (Method::PATCH, HttpMethod::Patch),
            (Method::HEAD, HttpMethod::Head),
            (Method::OPTIONS, HttpMethod::Options),
            (Method::CONNECT, HttpMethod::Connect),
            (Method::TRACE, HttpMethod::Trace),
        ];

        for (actix_method, expected_method) in methods {
            let request = create_test_client_request(
                actix_method.clone(),
                "http://example.com/test",
            );

            let result = get_downstream_request_metadata(&request);
            assert!(result.is_ok(), "Failed for method {:?}", actix_method);
            let (_, method) = result.unwrap();
            assert_eq!(method, expected_method);
        }
    }

    #[actix_web::test]
    async fn test_get_upstream_request_metadata_with_all_fields() {
        let req = TestRequest::default()
            .uri("/test/path")
            .method(Method::GET)
            .insert_header((
                DEFAULT_REQUEST_ID_KEY,
                "test-request-id-123".to_string(),
            ))
            .peer_addr("127.0.0.1:8080".parse().unwrap())
            .to_http_request();

        let result = get_upstream_request_metadata(&req);

        assert!(result.is_ok());
        let (path, request_id, client_ip) = result.unwrap();
        assert_eq!(path, "/test/path");
        assert_eq!(request_id, Some("test-request-id-123".to_string()));
        assert_eq!(client_ip, Some("127.0.0.1".to_string()));
    }

    #[actix_web::test]
    async fn test_get_upstream_request_metadata_without_optional_fields() {
        let req = TestRequest::default()
            .uri("/test/path")
            .method(Method::GET)
            .to_http_request();

        let result = get_upstream_request_metadata(&req);

        assert!(result.is_ok());
        let (path, request_id, client_ip) = result.unwrap();
        assert_eq!(path, "/test/path");
        assert_eq!(request_id, None);
        assert_eq!(client_ip, None);
    }

    #[actix_web::test]
    async fn test_get_upstream_request_metadata_different_paths() {
        let paths =
            vec!["/", "/api/v1/users", "/test/path/with/multiple/segments"];

        for path in paths {
            let req = TestRequest::default()
                .uri(path)
                .method(Method::GET)
                .to_http_request();

            let result = get_upstream_request_metadata(&req);
            assert!(result.is_ok());
            let (extracted_path, _, _) = result.unwrap();
            assert_eq!(extracted_path, path);
        }
    }

    #[actix_web::test]
    async fn test_get_upstream_request_metadata_client_ip_extraction() {
        let test_cases: Vec<(&str, Option<String>)> = vec![
            ("127.0.0.1:8080", Some("127.0.0.1".to_string())),
            ("192.168.1.1:3000", Some("192.168.1.1".to_string())),
            ("[::1]:8080", Some("::1".to_string())), // .ip() removes brackets for IPv6
        ];

        for (addr_str, expected_ip) in test_cases {
            let req = TestRequest::default()
                .uri("/test")
                .method(Method::GET)
                .peer_addr(addr_str.parse().unwrap())
                .to_http_request();

            let result = get_upstream_request_metadata(&req);
            assert!(result.is_ok());
            let (_, _, client_ip) = result.unwrap();
            assert_eq!(client_ip, expected_ip);
        }
    }

    #[actix_web::test]
    async fn test_get_upstream_request_metadata_request_id_variations() {
        // Test with valid request ID
        let req = TestRequest::default()
            .uri("/test")
            .method(Method::GET)
            .insert_header((
                DEFAULT_REQUEST_ID_KEY,
                "custom-request-id-456".to_string(),
            ))
            .to_http_request();

        let result = get_upstream_request_metadata(&req);
        assert!(result.is_ok());
        let (_, request_id, _) = result.unwrap();
        assert_eq!(request_id, Some("custom-request-id-456".to_string()));

        // Test without request ID header
        let req_no_id = TestRequest::default()
            .uri("/test")
            .method(Method::GET)
            .to_http_request();

        let result = get_upstream_request_metadata(&req_no_id);
        assert!(result.is_ok());
        let (_, request_id, _) = result.unwrap();
        assert_eq!(request_id, None);
    }

    #[actix_web::test]
    async fn test_get_downstream_request_metadata_different_urls() {
        let urls = vec![
            "http://example.com",
            "https://api.example.com/v1/users",
            "http://localhost:8080/api/test?param=value",
        ];

        for url in urls {
            let request = create_test_client_request(Method::GET, url);
            let result = get_downstream_request_metadata(&request);
            assert!(result.is_ok());
            let (extracted_url, _) = result.unwrap();
            assert_eq!(extracted_url, url);
        }
    }

    #[actix_web::test]
    async fn test_get_downstream_request_metadata_invalid_method() {
        // Note: This test verifies that valid methods are handled correctly In
        // practice, awc::ClientRequest should not allow invalid methods but we
        // test the conversion logic
        let valid_methods = vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::HEAD,
            Method::OPTIONS,
            Method::CONNECT,
            Method::TRACE,
        ];

        for method in valid_methods {
            let request = create_test_client_request(
                method.clone(),
                "http://example.com/test",
            );
            let result = get_downstream_request_metadata(&request);
            assert!(result.is_ok(), "Method {:?} should be valid", method);
        }
    }

    #[actix_web::test]
    async fn test_get_upstream_request_metadata_empty_path() {
        let req = TestRequest::default()
            .uri("/")
            .method(Method::GET)
            .to_http_request();

        let result = get_upstream_request_metadata(&req);
        assert!(result.is_ok());
        let (path, _, _) = result.unwrap();
        assert_eq!(path, "/");
    }

    #[actix_web::test]
    async fn test_get_upstream_request_metadata_path_with_query() {
        let req = TestRequest::default()
            .uri("/test/path?param1=value1&param2=value2")
            .method(Method::GET)
            .to_http_request();

        let result = get_upstream_request_metadata(&req);
        assert!(result.is_ok());
        let (path, _, _) = result.unwrap();
        // Should extract only the path, not the query string
        assert_eq!(path, "/test/path");
    }
}
