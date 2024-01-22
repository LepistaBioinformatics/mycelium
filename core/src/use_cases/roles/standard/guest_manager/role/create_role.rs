use crate::domain::{
    actors::DefaultActor,
    dtos::{profile::Profile, role::Role},
    entities::RoleRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};

/// Create a single role.
///
/// This function should be executed before the Guest Roles creation. Role
/// examples should be: ResultsExpert, CustomerExpert, Staff.
pub async fn create_role(
    profile: Profile,
    name: String,
    description: String,
    role_registration_repo: Box<&dyn RoleRegistration>,
) -> Result<GetOrCreateResponseKind<Role>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? ----------------------------------------------------------------------

    profile.get_default_create_ids_or_error(vec![
        DefaultActor::GuestManager.to_string(),
    ])?;

    // ? ----------------------------------------------------------------------
    // ? Persist Role
    // ? ----------------------------------------------------------------------

    role_registration_repo
        .get_or_create(Role {
            id: None,
            name,
            description,
        })
        .await
}
