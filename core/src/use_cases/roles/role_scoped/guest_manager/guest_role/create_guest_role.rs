use crate::domain::{
    actors::SystemActor,
    dtos::{
        guest_role::{GuestRole, Permission},
        profile::Profile,
    },
    entities::GuestRoleRegistration,
};

use mycelium_base::{
    dtos::Parent, entities::GetOrCreateResponseKind,
    utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Create a new guest role
///
/// This function should be called only by manager users. Roles should be
/// created after the application is registered by staff users why roles links
/// guest, permissions, and applications.
///
/// As example, a group of users need to has view only permissions to resources
/// of a single application. Thus, the role should include only the `View`
/// permission (level zero) for the `Movie` application. Thus, the role name
/// should be: "Movie Viewers".
#[tracing::instrument(name = "create_guest_role", skip_all)]
pub async fn create_guest_role(
    profile: Profile,
    name: String,
    description: String,
    role: Uuid,
    permission: Option<Permission>,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![SystemActor::GuestManager])?;

    // ? ----------------------------------------------------------------------
    // ? Persist UserRole
    // ? ----------------------------------------------------------------------

    guest_role_registration_repo
        .get_or_create(GuestRole::new(
            None,
            name,
            Some(description),
            Parent::Id(role),
            permission.unwrap_or(Permission::Read),
            None,
        ))
        .await
}
