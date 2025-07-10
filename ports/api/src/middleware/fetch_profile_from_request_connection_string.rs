use super::fetch_connection_string_from_request;
use crate::{
    dtos::MyceliumProfileData,
    middleware::recovery_profile_from_storage_engines,
};

use actix_web::HttpRequest;
use myc_core::domain::dtos::{
    security_group::PermissionedRole, token::UserAccountConnectionString,
};
use myc_http_tools::responses::GatewayError;
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "fetch_profile_from_request_connection_string",
    skip(req)
)]
pub(crate) async fn fetch_profile_from_request_connection_string(
    req: HttpRequest,
    tenant: Option<Uuid>,
    roles: Option<Vec<PermissionedRole>>,
) -> Result<MyceliumProfileData, GatewayError> {
    let span: tracing::Span = tracing::Span::current();

    tracing::trace!("Fetching profile from request connection string");

    // ? -----------------------------------------------------------------------
    // ? Extract the role scoped connection string
    // ? -----------------------------------------------------------------------

    let connection_string: UserAccountConnectionString =
        fetch_connection_string_from_request(req.clone())
            .instrument(span.to_owned())
            .await?
            .connection_string()
            .to_owned();

    // ? -----------------------------------------------------------------------
    // ? Check permissions intrinsic to the connection string
    // ? -----------------------------------------------------------------------

    //
    // If not None, filter the request tenant by the tenant stated in the
    // connection string
    //
    let updated_tenant = connection_string
        .get_tenant_id()
        .map(|tenant_id| {
            if tenant.is_none() {
                return Some(tenant_id);
            }

            if tenant.unwrap() == tenant_id {
                return Some(tenant_id);
            }

            None
        })
        .flatten();

    //
    // If not None, filter the request permissioned roles by roles stated in
    // the connection string
    //
    let updated_roles = connection_string
        .get_roles()
        .map(|connection_string_roles| {
            //
            // If the external permissioned roles are not provided, return the
            // connection string permissioned roles
            //
            if roles.is_none() {
                return Some(connection_string_roles);
            }

            //
            // If the external permissioned roles are provided, filter the
            // connection string permissioned roles by the roles stated in the
            // connection string
            //
            let mut filtered_permissioned_roles =
                connection_string_roles.clone();

            let local_pairs = roles
                .unwrap()
                .iter()
                .map(|role| (role.name.clone(), role.permission.clone()))
                .collect::<Vec<_>>();

            filtered_permissioned_roles.retain(|role| {
                local_pairs
                    .contains(&(role.name.clone(), role.permission.clone()))
            });

            match filtered_permissioned_roles.is_empty() {
                true => None,
                false => Some(filtered_permissioned_roles),
            }
        })
        .flatten();

    // ? -----------------------------------------------------------------------
    // ? Try to fetch profile from storage engines
    // ? -----------------------------------------------------------------------

    let profile = recovery_profile_from_storage_engines(
        req.clone(),
        connection_string.email.to_owned(),
        updated_tenant.to_owned(),
        updated_roles.to_owned(),
    )
    .instrument(span)
    .await?;

    // ? -----------------------------------------------------------------------
    // ? Return profile
    // ? -----------------------------------------------------------------------

    tracing::trace!("Profile: {:?}", profile.profile_redacted());

    Ok(MyceliumProfileData::from_profile(profile))
}
