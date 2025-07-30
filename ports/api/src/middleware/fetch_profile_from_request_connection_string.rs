use super::fetch_connection_string_from_request;
use crate::{
    dtos::MyceliumProfileData,
    middleware::recovery_profile_from_storage_engines,
};

use actix_web::HttpRequest;
use myc_core::domain::dtos::{
    security_group::PermissionedRole, token::UserAccountConnectionString,
};
use myc_http_tools::{responses::GatewayError, Permission};
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "fetch_profile_from_request_connection_string",
    skip(req)
)]
pub(crate) async fn fetch_profile_from_request_connection_string(
    req: HttpRequest,
    tenant: Option<Uuid>,
    required_roles: Option<Vec<PermissionedRole>>,
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
    let updated_tenant = filter_tenant(
        connection_string.get_tenant_id().to_owned(),
        tenant.to_owned(),
    );

    //
    // If not None, filter the request permissioned roles by roles stated in
    // the connection string
    //
    let updated_roles = filter_roles(
        required_roles.to_owned(),
        connection_string.get_roles().to_owned(),
    );

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

/// Filter tenant
///
/// Rules:
/// 1. If the connection string tenant is Some, return it.
/// 2. If the request tenant is Some and matches the connection string tenant, return it.
/// 3. If both are None, return None.
///
fn filter_tenant(
    connection_string_tenant: Option<Uuid>,
    request_tenant: Option<Uuid>,
) -> Option<Uuid> {
    if let Some(tenant) = connection_string_tenant {
        return Some(tenant);
    }

    if let Some(request_tenant) = request_tenant {
        return Some(request_tenant);
    }

    None
}

/// Filter roles from the profile and connection string
///
/// Rules:
/// 1. If both are Some, filter the profile roles by the connection string roles.
/// 2. If only the profile is Some, return the profile roles.
/// 3. If only the connection string is Some, return the connection string roles.
/// 4. If both are None, return None.
///
fn filter_roles(
    profile_roles: Option<Vec<PermissionedRole>>,
    connection_string_roles: Option<Vec<PermissionedRole>>,
) -> Option<Vec<PermissionedRole>> {
    //
    // Rule 1
    //
    if let (Some(profile_roles), Some(connection_string_roles)) =
        (profile_roles.to_owned(), connection_string_roles.to_owned())
    {
        let connection_string_roles_binding = connection_string_roles
            .iter()
            .map(|role| (role.name.clone(), role.permission.clone()))
            .collect::<Vec<_>>();

        //
        // The profile roles represented the real permissions of the user, when
        // the connection string restrictions are ignored. This permissions
        // should be filtered by the connection string restrictions.
        //
        let filtered_roles = profile_roles
            .iter()
            .filter(|profile_role| {
                //
                // If the connection string has no restrictions, the profile
                // permissions should be accepted.
                //
                if connection_string_roles.is_empty() {
                    return true;
                }

                let profile_perm =
                    profile_role.permission.clone().unwrap_or(Permission::Read);

                //
                // Otherwise, the profile permissions should be filtered by
                // the connection string permissions.
                //
                connection_string_roles_binding.iter().any(
                    |(name, permission)| {
                        let conn_str_perm =
                            permission.clone().unwrap_or(Permission::Read);

                        //
                        // Name should perfectly match
                        //
                        let name_match = name.to_lowercase()
                            == profile_role.name.to_lowercase();

                        //
                        // The profile permissions should be GREATER or EQUAL
                        // than the connection string permissions. The
                        // connection string permission represents the baseline.
                        //
                        let perm_match =
                            profile_perm.to_i32() >= conn_str_perm.to_i32();

                        name_match && perm_match
                    },
                )
            })
            .map(|i| i.to_owned())
            .collect::<Vec<_>>();

        return match filtered_roles.is_empty() {
            true => None,
            false => Some(filtered_roles),
        };
    }

    //
    // Rule 2
    //
    if let (Some(profile_roles), None) =
        (profile_roles.to_owned(), connection_string_roles.to_owned())
    {
        return Some(profile_roles);
    }

    //
    // Rule 3
    //
    if let (None, Some(connection_string_roles)) =
        (profile_roles, connection_string_roles)
    {
        return Some(connection_string_roles);
    }

    //
    // Rule 4
    //
    None
}
