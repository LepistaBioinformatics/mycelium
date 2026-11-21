mod answer_quota_outcome;
mod send_service_email_endpoint;

#[cfg(test)]
mod send_service_email_test_doubles;

pub use answer_quota_outcome::SendServiceEmailResponse;
pub use send_service_email_endpoint::*;

use actix_web::web;

/// Endpoints authenticated only by a notification connection string
///
/// Mounted at `/_adm/notifications` with no role header and no JWT
/// extractor: `fetch_notification_connection_string_from_request` is the
/// only gate.
pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config.service(send_service_email_url);
}
