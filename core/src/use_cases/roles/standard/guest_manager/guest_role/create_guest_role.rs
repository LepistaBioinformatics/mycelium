use crate::domain::{
    actors::DefaultActor,
    dtos::{
        guest::{GuestRole, Permissions},
        profile::Profile,
    },
    entities::GuestRoleRegistration,
};

use mycelium_base::{
    dtos::Parent, entities::GetOrCreateResponseKind,
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
#[tracing::instrument(name = "create_guest_role", skip_all)]
pub async fn create_guest_role(
    profile: Profile,
    name: String,
    description: String,
    role: Uuid,
    permissions: Option<Vec<Permissions>>,
    role_registration_repo: Box<&dyn GuestRoleRegistration>,
) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    profile.get_default_create_ids_or_error(vec![
        DefaultActor::GuestManager.to_string(),
    ])?;

    // ? ----------------------------------------------------------------------
    // ? Persist UserRole
    // ? ----------------------------------------------------------------------

    role_registration_repo
        .get_or_create(GuestRole {
            id: None,
            name,
            description: Some(description),
            role: Parent::Id(role),
            permissions: permissions.unwrap_or(vec![Permissions::View]),
        })
        .await
}
