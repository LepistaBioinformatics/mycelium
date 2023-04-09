use crate::domain::{
    dtos::{
        guest::{GuestRole, PermissionsType},
        profile::Profile,
    },
    entities::GuestRoleRegistration,
};

use clean_base::{
    dtos::enums::ParentEnum,
    entities::GetOrCreateResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
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
    profile: Profile,
    name: String,
    description: String,
    role: Uuid,
    permissions: Option<Vec<PermissionsType>>,
    role_registration_repo: Box<&dyn GuestRoleRegistration>,
) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
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
        return use_case_err(
            "The current user has no sufficient privileges to register new 
            guest-roles."
                .to_string(),
            Some(true),
            None,
        );
    }

    // ? ----------------------------------------------------------------------
    // ? Persist UserRole
    // ? ----------------------------------------------------------------------

    return role_registration_repo
        .get_or_create(GuestRole {
            id: None,
            name,
            description: Some(description),
            role: ParentEnum::Id(role),
            permissions,
        })
        .await;
}
