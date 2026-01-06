use crate::domain::{
    actors::SystemActor,
    dtos::{guest_role::GuestRole, security_group::SecurityGroup},
    entities::{GuestRoleRegistration, ServiceRead},
};

use mycelium_base::{
    entities::{FetchManyResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use std::{collections::HashMap, str::FromStr};

/// Propagate the declared roles to the SQL database.
///
/// This function propagates the declared roles to the SQL database. The roles
/// are propagated to the SQL database to allow the downstream services to
/// access the roles.
///
/// The function returns a tuple of the propagated roles.
///
#[tracing::instrument(
    name = "propagate_declared_roles_to_storage_engine",
    skip_all
)]
pub async fn propagate_declared_roles_to_storage_engine(
    services_fetching_repo: Box<&dyn ServiceRead>,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
) -> Result<(), MappedErrors> {
    //
    // Fetch all services and their routes to get the unique roles
    //
    let unique_roles = match services_fetching_repo
        .list_services(None, None, None)
        .await?
    {
        FetchManyResponseKind::Found(services) => services,
        _ => return use_case_err("No services found").as_error(),
    }
    .iter()
    .flat_map(|service| {
        service
            .routes
            .iter()
            .filter_map(|route| match &route.security_group {
                SecurityGroup::ProtectedByRoles(roles) => Some(roles),
                _ => None,
            })
    })
    .flatten()
    .map(|record| {
        GuestRole::new(
            None,
            record.name.to_owned(),
            None,
            record.permission.clone().unwrap_or_default(),
            None,
            false,
        )
    })
    .fold(HashMap::new(), |mut acc, role| {
        let key = (role.slug.clone(), role.permission.clone());
        acc.entry(key).or_insert(role);
        acc
    })
    .into_values()
    .collect::<Vec<GuestRole>>();

    for role in unique_roles {
        let system_actor = SystemActor::from_str(&role.slug).map_err(|e| {
            use_case_err(format!("Invalid system actor: {:?}", e))
        })?;

        //
        // System actors should only be created through dedicated use-cases. Do
        // not create them here
        //
        if !matches!(system_actor, SystemActor::CustomRole(_)) {
            tracing::trace!(
                "Skipping system actor '{:?}'. Please create manually using appropriate use-cases.", 
                system_actor
            );

            continue;
        }

        //
        // Proceed to create the role
        //
        match guest_role_registration_repo
            .get_or_create(role.to_owned())
            .await
        {
            Ok(res) => match res {
                GetOrCreateResponseKind::Created(role) => {
                    tracing::trace!("Role created: {:?}", role.slug);
                }
                GetOrCreateResponseKind::NotCreated(role, _) => {
                    tracing::trace!("Role already exists: {:?}", role.slug);
                }
            },
            Err(err) => {
                tracing::error!("Error creating role: {:?}", err);
            }
        }
    }

    Ok(())
}
