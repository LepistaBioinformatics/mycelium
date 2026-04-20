use crate::middleware::{BodyIdpContext, BodyIdpResolver, TelegramIdpResolver};

use actix_web::web;
use futures::StreamExt;
use myc_core::domain::dtos::{identity_source::IdentitySource, route::Route};
use myc_http_tools::responses::GatewayError;

/// Buffer the request payload and build a `BodyIdpContext` for routes that
/// resolve identity from the request body (`identity_source` is set).
///
/// The payload is a one-shot stream. For body-based IdP routes it is consumed
/// here, buffered, and the extracted `BodyIdpContext` + raw bytes are returned
/// so `stream_request_to_downstream` can replay the body via `send_body`.
///
/// For all other routes the payload is returned as-is and no buffering occurs.
#[tracing::instrument(name = "prepare_body_idp_context", skip_all)]
pub(super) async fn prepare_body_idp_context(
    route: &Route,
    payload: web::Payload,
) -> Result<
    (Option<BodyIdpContext>, Option<web::Bytes>, Option<web::Payload>),
    GatewayError,
> {
    let Some(ref source) = route.identity_source else {
        return Ok((None, None, Some(payload)));
    };

    let resolver = build_body_idp_resolver(source);
    let body = buffer_payload(payload).await?;
    let user_id = resolver.extract_user_id(&body)?;
    let ctx = BodyIdpContext { resolver, user_id };

    Ok((Some(ctx), Some(body), None))
}

fn build_body_idp_resolver(
    source: &IdentitySource,
) -> Box<dyn BodyIdpResolver> {
    match source {
        IdentitySource::Telegram => Box::new(TelegramIdpResolver),
    }
}

/// Collect a streaming `Payload` into a single `Bytes` value.
///
/// Capped at 512 KB to prevent DoS. Larger bodies are rejected with
/// `BadRequest`.
async fn buffer_payload(
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
