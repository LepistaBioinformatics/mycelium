use crate::domain::{
    dtos::{
        guest::{GuestRoleDTO, PermissionsType},
        profile::ProfileDTO,
    },
    entities::manager::guest_role_registration::GuestRoleRegistration,
};

use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// This function should be called only by manager users. Roles should be
/// created after the application is registered by staff users why roles links
/// guest, permissions, and applications.
///
/// As example, a group of users need to has view only permissions to resources
/// of a single application. Thus, the role should include only the `View`
/// permission (level zero) for the `Movie` application. Thus, the role name
/// should be: "Movie Viewers".
pub async fn create_guest_role(
    profile: ProfileDTO,
    name: String,
    description: String,
    role: Uuid,
    permissions: Option<Vec<PermissionsType>>,
    role_registration_repo: Box<&dyn GuestRoleRegistration>,
) -> Result<GetOrCreateResponseKind<GuestRoleDTO>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Collect permissions
    //
    // If permissions are None, their receives the default `View` only
    // permission.
    // ? ----------------------------------------------------------------------

    let permissions = permissions.unwrap_or(vec![PermissionsType::View]);

    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "The current user has no sufficient privileges to register new 
            guest-roles."
                .to_string(),
            Some(true),
            None,
        ));
    }

    // ? ----------------------------------------------------------------------
    // ? Persist UserRole
    // ? ----------------------------------------------------------------------

    return role_registration_repo
        .get_or_create(GuestRoleDTO {
            id: None,
            name,
            description,
            role: ParentEnum::Id(role),
            permissions,
            account: None,
        })
        .await;
}