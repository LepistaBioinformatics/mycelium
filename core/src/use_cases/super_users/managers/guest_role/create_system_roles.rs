use crate::domain::{
    actors::SystemActor::*,
    dtos::{
        guest_role::{GuestRole, Permission},
        profile::Profile,
        role::Role,
    },
    entities::{GuestRoleRegistration, RoleRegistration},
};

use mycelium_base::{
    dtos::Parent, entities::GetOrCreateResponseKind,
    utils::errors::MappedErrors,
};
use tracing::error;

#[tracing::instrument(
    name = "create_system_roles",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, role_registration_repo, guest_role_registration_repo)
)]
pub async fn create_system_roles(
    profile: Profile,
    role_registration_repo: Box<&dyn RoleRegistration>,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
) -> Result<Vec<GuestRole>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Batch create roles
    // ? -----------------------------------------------------------------------

    let mut roles_creation_responses: Vec<(
        GetOrCreateResponseKind<Role>,
        String,
    )> = vec![];

    for (actor, description) in [
        (
            SubscriptionsManager,
            "Actors who manage subscription accounts",
        ),
        (UsersManager, "Actors who manage user accounts"),
        (AccountManager, "Actors who manage single account settings"),
        (GuestManager, "Actors who perform guest actions"),
        (GatewayManager, "Actors who manage gateway settings"),
        (SystemManager, "Actors who manage system settings"),
        (TenantManager, "Actors who manage single tenant settings"),
    ] {
        //
        // Split actor name by hyphen or underscore and capitalize the first
        // letter of all words and join all the words by space.
        //
        let _actor = actor
            .to_string()
            .split(|c| c == '-' || c == '_')
            .map(|s| {
                s.chars().next().unwrap().to_uppercase().to_string() + &s[1..]
            })
            .collect::<Vec<_>>()
            .join(" ");

        //
        // Create role
        //
        let response = role_registration_repo
            .get_or_create(Role::new(None, _actor, description.to_string()))
            .await?;

        roles_creation_responses.push((response, description.to_string()));
    }

    let roles_parsed_responses = roles_creation_responses
        .iter()
        .map(|(response, description)| {
            let role = match response {
                GetOrCreateResponseKind::Created(role) => role,
                GetOrCreateResponseKind::NotCreated(role, _) => role,
            };

            (role, description.to_owned())
        })
        .collect::<Vec<_>>();

    // ? -----------------------------------------------------------------------
    // ? Batch create guest-roles
    // ? -----------------------------------------------------------------------

    let mut guest_roles_creation_responses: Vec<(
        GetOrCreateResponseKind<GuestRole>,
        String,
    )> = vec![];

    for (role, description) in roles_parsed_responses {
        let role_id = match role.id {
            Some(id) => id,
            None => {
                error!("Role ID not found for role: {:?}", role);

                continue;
            }
        };

        for (permission, alias) in [
            (Permission::Read, "Reader"),
            (Permission::Write, "Writer"),
            (Permission::ReadWrite, "Reader-Writer"),
        ] {
            let response = guest_role_registration_repo
                .get_or_create(GuestRole::new(
                    None,
                    format!("{} {}", role.name, alias),
                    Some(format!(
                        "{} with {} permissions",
                        description,
                        permission.to_string()
                    )),
                    Parent::Id(role_id),
                    permission,
                    None,
                ))
                .await?;

            guest_roles_creation_responses
                .push((response, description.clone()));
        }
    }

    let guest_roles_parsed_responses = guest_roles_creation_responses
        .iter()
        .map(|(response, _)| match response {
            GetOrCreateResponseKind::Created(role) => role.to_owned(),
            GetOrCreateResponseKind::NotCreated(role, _) => role.to_owned(),
        })
        .collect::<Vec<_>>();

    Ok(guest_roles_parsed_responses)
}
