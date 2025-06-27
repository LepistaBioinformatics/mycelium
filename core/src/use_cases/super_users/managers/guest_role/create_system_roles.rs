use crate::domain::{
    actors::SystemActor::*,
    dtos::{
        guest_role::{GuestRole, Permission},
        profile::Profile,
    },
    entities::GuestRoleRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use tracing::{error, info, trace, warn};

/// Create system roles
///
/// System roles should be used to attribute permissions to actors who manage
/// specific parts of the system. This function creates the following roles:
///
/// - Subscriptions Manager
/// - Users Manager
/// - Account Manager
/// - Guest Manager
/// - Gateway Manager
/// - System Manager
/// - Tenant Manager
///
#[tracing::instrument(
    name = "create_system_roles",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, guest_role_registration_repo)
)]
pub async fn create_system_roles(
    profile: Profile,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
) -> Result<Vec<GuestRole>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Batch create guest-roles
    // ? -----------------------------------------------------------------------

    let mut guest_roles_creation_responses: Vec<(
        GetOrCreateResponseKind<GuestRole>,
        String,
    )> = vec![];

    for (actor, description) in [
        (
            SubscriptionsManager,
            "Actors who manage subscription accounts",
        ),
        (UsersManager, "Actors who manage user accounts"),
        (AccountManager, "Actors who manage single account settings"),
        (GuestsManager, "Actors who perform guest actions"),
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

        for permission in [Permission::Read, Permission::Write] {
            trace!(
                "Creating role for {} with {} permissions",
                _actor,
                permission.to_string()
            );

            let response = match guest_role_registration_repo
                .get_or_create(GuestRole::new(
                    None,
                    _actor.to_owned(),
                    Some(format!(
                        "{} with {} permissions",
                        description,
                        permission.to_string()
                    )),
                    permission.to_owned(),
                    None,
                    true,
                ))
                .await
            {
                Ok(response) => response,
                Err(e) => {
                    error!(
                        "Failed to create role for {} with {} permissions",
                        _actor,
                        permission.to_string()
                    );

                    return Err(e);
                }
            };

            guest_roles_creation_responses
                .push((response, description.to_string()));
        }
    }

    let guest_roles_parsed_responses = guest_roles_creation_responses
        .iter()
        .map(|(response, _)| match response {
            GetOrCreateResponseKind::Created(role) => {
                info!(
                    "Role {} with {} permissions created",
                    role.name,
                    role.permission.to_string()
                );

                role.to_owned()
            }
            GetOrCreateResponseKind::NotCreated(role, msg) => {
                warn!(
                    "Role {} with {} permissions not created due to: {}",
                    role.name,
                    role.permission.to_string(),
                    msg
                );

                role.to_owned()
            }
        })
        .collect::<Vec<_>>();

    Ok(guest_roles_parsed_responses)
}
