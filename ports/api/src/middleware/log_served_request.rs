use crate::models::requester_identity::RequesterIdentity;

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error, HttpMessage,
};
use myc_http_tools::settings::DEFAULT_REQUEST_ID_KEY;
use std::time::Instant;

/// Health probes and API docs: frequent or static, logging them would bury
/// the requests that matter.
const UNLOGGED_PATH_PREFIXES: [&str; 3] =
    ["/health", "/doc/swagger", "/doc/redoc"];

/// The gateway access log: one `info` line per served request with what was
/// asked, the outcome and who asked. Identity resolution itself logs only at
/// `debug`.
pub(crate) async fn log_served_request(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    if UNLOGGED_PATH_PREFIXES
        .iter()
        .any(|prefix| req.path().starts_with(prefix))
    {
        return next.call(req).await;
    }

    let started = Instant::now();
    let method = req.method().to_string();
    let path = req.path().to_owned();
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("-")
        .to_owned();
    let request_id = req
        .headers()
        .get(DEFAULT_REQUEST_ID_KEY)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let response = next.call(req).await?;

    let requester = response
        .request()
        .extensions()
        .get::<RequesterIdentity>()
        .cloned()
        .unwrap_or_default();

    tracing::info!(
        stage = "request.served",
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        duration_ms = started.elapsed().as_millis() as u64,
        request_id = request_id.as_deref().unwrap_or("-"),
        client_ip = %client_ip,
        requester.account_id =
            %requester.account_id.map(|id| id.to_string()).unwrap_or("-".into()),
        requester.email = requester.email.as_deref().unwrap_or("-"),
        "Request served"
    );

    Ok(response)
}
