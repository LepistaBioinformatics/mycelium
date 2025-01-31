use crate::{
    dtos::MyceliumProfileData,
    middleware::check_credentials_with_multi_identity_provider,
};

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::dtos::route_type::PermissionedRoles,
    use_cases::service::profile::{fetch_profile_from_email, ProfileResponse},
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::responses::GatewayError;
use shaku::HasComponent;
use uuid::Uuid;

/// Try to populate profile to request header
///
/// This function is auxiliary of the MyceliumProfileData struct used to extract
/// the Mycelium Profile from the request on mycelium native APIs.
#[tracing::instrument(name = "fetch_profile_from_request", skip(req))]
pub(crate) async fn fetch_profile_from_request(
    req: HttpRequest,
    tenant: Option<Uuid>,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
) -> Result<MyceliumProfileData, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Build dependencies
    // ? -----------------------------------------------------------------------

    let app_module = match req.app_data::<web::Data<SqlAppModule>>() {
        Some(module) => module,
        None => {
            tracing::error!(
                "Unable to extract profile fetching module from request"
            );

            return Err(GatewayError::InternalServerError(
                "Unexpected error on get profile".to_string(),
            ));
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Profile Fetching
    // ? -----------------------------------------------------------------------

    let email =
        check_credentials_with_multi_identity_provider(req.clone()).await?;

    if email.is_none() {
        return Err(GatewayError::Unauthorized(format!(
            "Unable o extract user identity from request."
        )));
    }

    if let Some(email) = email.to_owned() {
        tracing::trace!("Email: {:?}", email.redacted_email());
    };

    let profile = match fetch_profile_from_email(
        email.to_owned().unwrap(),
        None,
        tenant,
        roles,
        permissioned_roles,
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Err(err) => {
            tracing::warn!(
                "Unexpected error on fetch profile from email: {err}"
            );

            return Err(GatewayError::InternalServerError(
                "Unexpected error on fetch profile from email.".to_string(),
            ));
        }
        Ok(res) => match res {
            ProfileResponse::UnregisteredUser(email) => {
                return Err(GatewayError::Forbidden(format!(
                    "Unauthorized access: {email}",
                    email = email.email(),
                )))
            }
            ProfileResponse::RegisteredUser(res) => res,
        },
    };

    tracing::trace!("Profile: {:?}", profile.profile_redacted());

    Ok(MyceliumProfileData::from_profile(profile))
}
