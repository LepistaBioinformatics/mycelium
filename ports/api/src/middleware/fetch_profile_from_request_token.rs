use crate::{
    dtos::MyceliumProfileData,
    middleware::{
        check_credentials_with_multi_identity_provider,
        recovery_profile_from_storage_engines,
    },
};

use actix_web::HttpRequest;
use myc_core::domain::dtos::security_group::PermissionedRole;
use myc_http_tools::responses::GatewayError;
use tracing::Instrument;
use uuid::Uuid;

/// Try to populate profile to request header
///
/// This function is auxiliary of the MyceliumProfileData struct used to extract
/// the Mycelium Profile from the request on mycelium native APIs.
#[tracing::instrument(name = "fetch_profile_from_request_token", skip(req))]
pub(crate) async fn fetch_profile_from_request_token(
    req: HttpRequest,
    tenant: Option<Uuid>,
    roles: Option<Vec<PermissionedRole>>,
) -> Result<MyceliumProfileData, GatewayError> {
    let span = tracing::Span::current();

    tracing::trace!("Fetching profile from request token");

    // ? -----------------------------------------------------------------------
    // ? Fetch email from request
    // ? -----------------------------------------------------------------------

    let (email, _) =
        check_credentials_with_multi_identity_provider(req.clone()).await?;

    tracing::trace!("Email: {:?}", email.redacted_email());

    // ? -----------------------------------------------------------------------
    // ? Try to fetch profile from storage engines
    // ? -----------------------------------------------------------------------

    let profile = recovery_profile_from_storage_engines(
        req.clone(),
        email.to_owned(),
        tenant,
        roles.to_owned(),
    )
    .instrument(span)
    .await?;

    // ? -----------------------------------------------------------------------
    // ? Return profile
    // ? -----------------------------------------------------------------------

    tracing::trace!("Profile: {:?}", profile.profile_redacted());

    Ok(MyceliumProfileData::from_profile(profile))
}
