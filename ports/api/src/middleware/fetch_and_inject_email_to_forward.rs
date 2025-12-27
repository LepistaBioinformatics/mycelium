use super::check_credentials_with_multi_identity_provider;

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_http_tools::{responses::GatewayError, settings::DEFAULT_EMAIL_KEY};
use opentelemetry::{global, KeyValue};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;
use tracing::Instrument;

/// Fetch and inject email to forward
///
/// This function is used to fetch the user email from the request and inject it
/// into the request headers.
///
#[tracing::instrument(name = "fetch_and_inject_email_to_forward", skip_all)]
pub async fn fetch_and_inject_email_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
    service_name: String,
) -> Result<ClientRequest, GatewayError> {
    let span = tracing::Span::current();

    tracing::trace!("Injecting email to forward");

    let (email, _) =
        check_credentials_with_multi_identity_provider(req.clone())
            .instrument(span.to_owned())
            .await?;

    // Get a meter
    let meter = global::meter("router_counter");

    // Create a metric
    let counter = meter.u64_counter("router.requests_count").build();

    counter.add(
        1,
        &[
            KeyValue::new(
                "identifier",
                format!("email:{}", email.encoded_email()),
            ),
            KeyValue::new("down_service_name", service_name),
        ],
    );

    forwarded_req.headers_mut().insert(
        HeaderName::from_str(DEFAULT_EMAIL_KEY).unwrap(),
        match HeaderValue::from_str(
            &serde_json::to_string(&email.email()).unwrap(),
        ) {
            Err(err) => {
                tracing::warn!("err: {:?}", err.to_string());
                return Err(GatewayError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(res) => res,
        },
    );

    span.record("myc.router.email", &Some(email.redacted_email()));

    tracing::trace!("Email injected to forward");

    Ok(forwarded_req)
}
