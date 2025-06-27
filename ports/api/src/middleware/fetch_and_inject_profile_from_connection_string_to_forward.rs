use super::fetch_connection_string_from_request;

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::{
    domain::{
        dtos::{
            security_group::PermissionedRoles,
            token::UserAccountConnectionString,
        },
        entities::{LicensedResourcesFetching, ProfileFetching},
    },
    use_cases::service::profile::{fetch_profile_from_email, ProfileResponse},
};
use myc_http_tools::{responses::GatewayError, settings::DEFAULT_PROFILE_KEY};
use reqwest::header::{HeaderName, HeaderValue};
use std::str::FromStr;
use tracing::error;

#[tracing::instrument(
    name = "fetch_and_inject_profile_from_connection_string_to_forward",
    skip_all
)]
pub(crate) async fn fetch_and_inject_profile_from_connection_string_to_forward(
    req: HttpRequest,
    mut forwarded_req: ClientRequest,
    roles: Option<Vec<String>>,
    permissioned_roles: Option<PermissionedRoles>,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<ClientRequest, GatewayError> {
    // ? -----------------------------------------------------------------------
    // ? Extract the role scoped connection string
    // ? -----------------------------------------------------------------------

    let connection_string: UserAccountConnectionString =
        fetch_connection_string_from_request(req)
            .await?
            .connection_string()
            .to_owned();

    // ? -----------------------------------------------------------------------
    // ? Fetch profile from owner id
    // ? -----------------------------------------------------------------------

    let profile = match fetch_profile_from_email(
        connection_string.email.to_owned(),
        None,
        None,
        roles.clone(),
        permissioned_roles.clone(),
        profile_fetching_repo,
        licensed_resources_fetching_repo,
    )
    .await
    .map_err(|err| {
        GatewayError::InternalServerError(format!(
            "Unexpected error while fetching profile: {err}"
        ))
    })? {
        ProfileResponse::RegisteredUser(profile) => profile,
        ProfileResponse::UnregisteredUser(_) => {
            return Err(GatewayError::Unauthorized(
                "Unauthorized to access resource".to_string(),
            ));
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Inject the serialized connection string into the forwarded request
    // ? -----------------------------------------------------------------------

    forwarded_req.headers_mut().insert(
        HeaderName::from_str(DEFAULT_PROFILE_KEY).unwrap(),
        match HeaderValue::from_str(&serde_json::to_string(&profile).unwrap()) {
            Err(err) => {
                error!("err: {:?}", err.to_string());
                return Err(GatewayError::InternalServerError(format!(
                    "{err}"
                )));
            }
            Ok(res) => res,
        },
    );

    Ok(forwarded_req)
}
