use crate::middleware::{
    fetch_and_inject_email_to_forward,
    fetch_and_inject_profile_from_token_to_forward,
};

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::{
    callback::UserInfo, route::Route, security_group::SecurityGroup,
};
use myc_http_tools::{
    responses::GatewayError, settings::MYCELIUM_SECURITY_GROUP,
};
use mycelium_base::dtos::Parent;
use tracing::Instrument;

/// Check the security group
///
/// This function checks the security group of the route and injects the
/// appropriate headers into the request.
///
#[tracing::instrument(
    name = "check_security_group",
    fields(
        request_path = format!("{} {}", req.method(), req.path()),
        route_pattern = format!("'{}'", route.path),
        security_group = %route.security_group.to_string(),
    ),
    skip(req, downstream_request, route)
)]
pub(super) async fn check_security_group(
    req: HttpRequest,
    mut downstream_request: ClientRequest,
    route: Route,
) -> Result<(ClientRequest, SecurityGroup, Option<UserInfo>), GatewayError> {
    let span = tracing::Span::current();

    let security_group = route.security_group.to_owned();

    tracing::info!(
        stage = "router.check_security_group",
        security_group = %security_group.to_string(),
        "Security group check started"
    );

    //
    // Inject security group as header
    //
    let serialized_security_group =
        serde_json::to_string(&security_group.to_owned()).map_err(|e| {
            tracing::error!("Failed to serialize security group: {e}");

            GatewayError::BadGateway(
                "Failed to build downstream request. Please try again later."
                    .to_string(),
            )
        })?;

    downstream_request = downstream_request
        .insert_header((MYCELIUM_SECURITY_GROUP, serialized_security_group));

    let service_name = match route.service {
        Parent::Record(ref service) => service.name.to_owned(),
        Parent::Id(_) => {
            tracing::info!(
                stage = "router.check_security_group",
                outcome = "error",
                "Service not found; security group check aborted"
            );
            tracing::error!("Service not found");

            return Err(GatewayError::InternalServerError(String::from(
                "Service not found",
            )));
        }
    };

    //
    // Check requester permissions given the security group
    //
    let (new_downstream_request, user_info) = match security_group.to_owned() {
        //
        // Public routes do not need any authentication or profile injection.
        //
        SecurityGroup::Public => {
            tracing::info!(
                stage = "router.check_security_group",
                security_group = "Public",
                outcome = "ok",
                "Public route; check completed"
            );
            (downstream_request, None)
        }
        //
        // Authenticated routes should include the user email into the request
        // token
        //
        SecurityGroup::Authenticated => {
            tracing::info!(
                stage = "router.identity_resolution",
                "Identity (email) resolution started"
            );
            let (mod_downstream_request, email) =
                fetch_and_inject_email_to_forward(
                    req,
                    downstream_request,
                    service_name,
                )
                .instrument(span.to_owned())
                .await?;
            tracing::info!(
                stage = "router.identity_resolution",
                outcome = "ok",
                "Identity (email) resolution completed"
            );
            (mod_downstream_request, Some(UserInfo::new_email(email)))
        }
        //
        // Protected routes should include the full qualified user profile into
        // the header
        //
        SecurityGroup::Protected => {
            tracing::info!(
                stage = "router.profile_resolution",
                "Profile resolution started"
            );
            let (mod_downstream_request, profile) =
                fetch_and_inject_profile_from_token_to_forward(
                    req,
                    downstream_request,
                    None,
                    None,
                    service_name,
                )
                .instrument(span.to_owned())
                .await?;
            tracing::info!(
                stage = "router.profile_resolution",
                outcome = "ok",
                "Profile resolution completed"
            );
            (mod_downstream_request, Some(UserInfo::new_profile(profile)))
        }
        //
        // Protected routes should include the user profile filtered by roles
        // into the header
        //
        SecurityGroup::ProtectedByRoles(roles) => {
            tracing::info!(
                stage = "router.profile_resolution",
                "Profile resolution by roles started"
            );
            let (mod_downstream_request, profile) =
                fetch_and_inject_profile_from_token_to_forward(
                    req,
                    downstream_request,
                    None,
                    Some(roles),
                    service_name,
                )
                .instrument(span.to_owned())
                .await?;
            tracing::info!(
                stage = "router.profile_resolution",
                outcome = "ok",
                "Profile resolution by roles completed"
            );
            (mod_downstream_request, Some(UserInfo::new_profile(profile)))
        }
    };

    tracing::info!(
        stage = "router.check_security_group",
        outcome = "ok",
        security_group = %security_group.to_string(),
        "Security group check completed"
    );

    Ok((new_downstream_request, security_group, user_info))
}
