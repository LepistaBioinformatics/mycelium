use actix_web::web;
use futures::StreamExt;
use myc_core::domain::dtos::{
    http::HttpMethod, route_retry_policy::RouteRetryPolicy,
};
use myc_http_tools::responses::GatewayError;

/// Buffer the request body when the route may send it more than once.
///
/// A streamed payload can be sent a single time. Routes with a retry policy
/// that applies to the request method need the body in memory to replay it on
/// each attempt. Bodies already buffered (body-based IdP routes) are reused.
#[tracing::instrument(name = "buffer_request_body_for_retry", skip_all)]
pub(super) async fn buffer_request_body_for_retry(
    retry: Option<&RouteRetryPolicy>,
    method: &HttpMethod,
    pre_buffered_body: Option<web::Bytes>,
    payload: Option<web::Payload>,
) -> Result<(Option<web::Bytes>, Option<web::Payload>), GatewayError> {
    let replay_needed = retry.is_some_and(|policy| policy.applies_to(method));

    match (pre_buffered_body, payload) {
        (None, Some(payload)) if replay_needed => {
            Ok((Some(buffer_payload(payload).await?), None))
        }
        (pre_buffered_body, payload) => Ok((pre_buffered_body, payload)),
    }
}

/// Collect a streaming `Payload` into a single `Bytes` value.
///
/// Capped at 512 KB to prevent DoS. Larger bodies are rejected with
/// `BadRequest`.
pub(super) async fn buffer_payload(
    payload: web::Payload,
) -> Result<web::Bytes, GatewayError> {
    const MAX_BODY: usize = 512 * 1024;

    let mut payload = payload;
    let mut body = web::BytesMut::with_capacity(8192);

    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(|e: actix_web::error::PayloadError| {
            GatewayError::BadRequest(format!("Payload read error: {e}"))
        })?;

        if body.len() + chunk.len() > MAX_BODY {
            return Err(GatewayError::BadRequest(
                "Request body exceeds 512 KB limit".to_string(),
            ));
        }

        body.extend_from_slice(&chunk);
    }

    Ok(body.freeze())
}
