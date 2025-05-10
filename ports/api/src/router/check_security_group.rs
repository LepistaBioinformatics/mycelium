use crate::middleware::{
    fetch_and_inject_email_to_forward, fetch_and_inject_profile_to_forward,
    fetch_and_inject_role_scoped_connection_string_to_forward,
};

use actix_web::HttpRequest;
use awc::ClientRequest;
use myc_core::domain::dtos::{route::Route, security_group::SecurityGroup};
use myc_http_tools::responses::GatewayError;
use tracing::Instrument;

/// Check the security group
///
/// This function checks the security group of the route and injects the
/// appropriate headers into the request.
///
#[tracing::instrument(name = "check_security_group", skip_all)]
pub(super) async fn check_security_group(
    req: HttpRequest,
    mut downstream_request: ClientRequest,
    route: Route,
) -> Result<ClientRequest, GatewayError> {
    let span = tracing::Span::current();

    match route.security_group.to_owned() {
        //
        // Public routes do not need any authentication or profile injection.
        //
        SecurityGroup::Public => (),
        SecurityGroup::Authenticated => {
            //
            // Try to extract user email from the request and inject it into the
            // request headers
            //
            downstream_request =
                fetch_and_inject_email_to_forward(req, downstream_request)
                    .instrument(span.to_owned())
                    .await?;
        }
        //
        // Protected routes should include the full qualified user profile into
        // the header
        //
        SecurityGroup::Protected => {
            //
            // Try to populate profile from the request
            //
            downstream_request = fetch_and_inject_profile_to_forward(
                req,
                downstream_request,
                None,
                None,
                None,
            )
            .instrument(span.to_owned())
            .await?;
        }
        //
        // Protected routes should include the user profile filtered by roles
        // into the header
        //
        SecurityGroup::ProtectedByRoles { roles } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles
            //
            downstream_request = fetch_and_inject_profile_to_forward(
                req,
                downstream_request,
                None,
                Some(roles),
                None,
            )
            .instrument(span.to_owned())
            .await?;
        }
        //
        // Protected routes should include the user profile filtered by roles
        // and permissions into the header
        //
        SecurityGroup::ProtectedByPermissionedRoles { permissioned_roles } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles and permissions
            //
            downstream_request = fetch_and_inject_profile_to_forward(
                req,
                downstream_request,
                None,
                None,
                Some(permissioned_roles),
            )
            .instrument(span.to_owned())
            .await?;
        }
        //
        // Protected routes by service token should include the users role which
        // the service token is associated
        //
        SecurityGroup::ProtectedByServiceTokenWithRole { roles } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles and permissions
            //
            downstream_request =
                fetch_and_inject_role_scoped_connection_string_to_forward(
                    req,
                    downstream_request,
                    Some(roles),
                    None,
                )
                .instrument(span.to_owned())
                .await?;
        }
        //
        // Protected routes by service token should include the users role which
        // the service token is associated
        //
        SecurityGroup::ProtectedByServiceTokenWithPermissionedRoles {
            permissioned_roles,
        } => {
            //
            // Try to populate profile from the request filtering licensed
            // resources by roles and permissions
            //
            downstream_request =
                fetch_and_inject_role_scoped_connection_string_to_forward(
                    req,
                    downstream_request,
                    None,
                    Some(permissioned_roles),
                )
                .instrument(span.to_owned())
                .await?;
        }
    }

    Ok(downstream_request)
}
