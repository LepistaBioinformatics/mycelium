use crate::{models::ProfileSignatureVerification, responses::GatewayError};

use actix_web::{web, HttpRequest};
use std::sync::OnceLock;

/// Generic detail of a verification configuration failure (never the cause)
pub(crate) const SIGNATURE_INTERNAL_DETAIL: &str =
    "Unable to check user identity. Please contact administrators";

/// Process-wide settings loaded once when no `web::Data` was registered
///
/// Holds the outcome, including a configuration error, so the environment is
/// never re-read per request and an invalid setting keeps failing closed.
///
static PROCESS_VERIFICATION: OnceLock<
    Result<ProfileSignatureVerification, ()>,
> = OnceLock::new();

/// The verification settings that apply to this request
///
/// Prefers the `web::Data<ProfileSignatureVerification>` registered by the
/// service; otherwise loads the environment once. An invalid configuration
/// is a 500 on every request, never a silent downgrade to "disabled".
///
pub(crate) fn resolve_profile_signature_verification(
    req: &HttpRequest,
) -> Result<ProfileSignatureVerification, GatewayError> {
    if let Some(registered) =
        req.app_data::<web::Data<ProfileSignatureVerification>>()
    {
        return Ok(registered.get_ref().clone());
    }

    PROCESS_VERIFICATION
        .get_or_init(load_from_environment)
        .clone()
        .map_err(|_| {
            GatewayError::InternalServerError(
                SIGNATURE_INTERNAL_DETAIL.to_string(),
            )
        })
}

fn load_from_environment() -> Result<ProfileSignatureVerification, ()> {
    ProfileSignatureVerification::from_environment().map_err(|error| {
        tracing::error!(
            code = "MYC00035",
            reason = %error.msg(),
            "profile_signature.verify outcome=error reason=configuration_invalid"
        );
    })
}
