use actix_web::{http::header::RETRY_AFTER, HttpResponse};
use myc_core::domain::dtos::{
    native_error_codes::NativeErrorCodes, notification_dispatch::QuotaOutcome,
};
use myc_http_tools::utils::HttpJsonResponse;
use serde::Serialize;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(Serialize, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct SendServiceEmailResponse {
    /// One id per recipient, in request order. Each id is also sent in the
    /// `X-Mycelium-Message-Id` mail header.
    pub message_ids: Vec<Uuid>,
}

/// Answer the client with the quota outcome of a send
///
/// - `Accepted`: 202 with the message ids;
/// - `Exceeded`: 429 with MYC00040 and a `Retry-After` header, in seconds.
pub(crate) fn answer_quota_outcome(outcome: QuotaOutcome) -> HttpResponse {
    match outcome {
        QuotaOutcome::Accepted(message_ids) => HttpResponse::Accepted()
            .json(SendServiceEmailResponse { message_ids }),
        QuotaOutcome::Exceeded { retry_after_secs } => {
            HttpResponse::TooManyRequests()
                .insert_header((RETRY_AFTER, retry_after_secs.to_string()))
                .json(
                    HttpJsonResponse::new_message(
                        "Service notification quota exceeded",
                    )
                    .with_code(NativeErrorCodes::MYC00040),
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

    #[test]
    fn accepted_is_202() {
        let response =
            answer_quota_outcome(QuotaOutcome::Accepted(vec![Uuid::new_v4()]));

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[test]
    fn exceeded_is_429_with_retry_after_in_seconds() {
        let response = answer_quota_outcome(QuotaOutcome::Exceeded {
            retry_after_secs: 90,
        });

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(RETRY_AFTER).unwrap(), "90");
    }
}
