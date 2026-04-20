use crate::middleware::{
    fetch_and_inject_email_to_forward,
    fetch_and_inject_profile_from_body_idp,
    fetch_and_inject_profile_from_token_to_forward,
    BodyIdpContext,
};

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::{
    callback::UserInfo,
    route::Route,
    security_group::SecurityGroup,
    service::Service,
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
    skip(req, downstream_request, route, body_idp)
)]
pub(super) async fn check_security_group(
    req: HttpRequest,
    mut downstream_request: ClientRequest,
    route: Route,
    body_idp: Option<BodyIdpContext>,
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

    // ? -----------------------------------------------------------------------
    // ? Body-based IdP auth — identity resolved from request body
    // ? -----------------------------------------------------------------------

    if let Some(ctx) = body_idp {
        return authenticate_from_body_idp(
            req,
            downstream_request,
            security_group,
            ctx,
            &route.service,
        )
        .instrument(span)
        .await;
    }

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

#[tracing::instrument(
    name = "authenticate_from_body_idp",
    skip_all,
    fields(user_id = ctx.user_id.as_str())
)]
async fn authenticate_from_body_idp(
    req: HttpRequest,
    downstream_request: ClientRequest,
    security_group: SecurityGroup,
    ctx: BodyIdpContext,
    service: &Parent<Service, uuid::Uuid>,
) -> Result<(ClientRequest, SecurityGroup, Option<UserInfo>), GatewayError> {
    let service_record = match service {
        Parent::Record(ref s) => s,
        Parent::Id(_) => {
            return Err(GatewayError::InternalServerError(
                "Service not loaded for body IdP identity resolution"
                    .to_string(),
            ))
        }
    };

    tracing::info!(
        stage = "router.body_idp_auth",
        user_id = ctx.user_id.as_str(),
        "Resolving profile from body IdP user_id"
    );

    let (new_downstream_request, profile) =
        fetch_and_inject_profile_from_body_idp(
            req,
            downstream_request,
            ctx,
            service_record,
            &security_group,
        )
        .await?;

    tracing::info!(
        stage = "router.body_idp_auth",
        outcome = "ok",
        "Body IdP auth: profile injected"
    );

    Ok((
        new_downstream_request,
        security_group,
        Some(UserInfo::new_profile(profile)),
    ))
}
