use super::buffer_request_body::buffer_payload;
use crate::middleware::{BodyIdpContext, BodyIdpResolver, TelegramIdpResolver};

use actix_web::web;
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
    (
        Option<BodyIdpContext>,
        Option<web::Bytes>,
        Option<web::Payload>,
    ),
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
