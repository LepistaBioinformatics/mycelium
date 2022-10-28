use crate::domain::{
    dtos::{
        enums::ParentEnum,
        guest::{PermissionsType, UserRoleDTO},
    },
    entities::{
        manager::user_role_registration::UserRoleRegistration,
        shared::default_responses::GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
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
pub async fn create_role(
    name: String,
    description: String,
    application: Uuid,
    permissions: Option<Vec<PermissionsType>>,
    role_registration_repo: Box<&dyn UserRoleRegistration>,
) -> Result<GetOrCreateResponse<UserRoleDTO>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Collect permissions
    //
    // If permissions are None, their receives the default `View` only
    // permission.
    // ? ----------------------------------------------------------------------

    let permissions = permissions.unwrap_or(vec![PermissionsType::View]);

    // ? ----------------------------------------------------------------------
    // ? Persist UserRole
    // ? ----------------------------------------------------------------------

    return role_registration_repo
        .get_or_create(UserRoleDTO {
            id: None,
            name,
            description,
            application: ParentEnum::Id(application),
            permissions,
        })
        .await;
}
