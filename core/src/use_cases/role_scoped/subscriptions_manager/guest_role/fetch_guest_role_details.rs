use crate::domain::{
    actors::SystemActor,
    dtos::{guest_role::GuestRole, profile::Profile},
    entities::GuestRoleFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

/// List guest roles
#[tracing::instrument(name = "fetch_guest_role_details", skip_all)]
pub async fn fetch_guest_role_details(
    profile: Profile,
    guest_role_id: Uuid,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::SubscriptionsManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch Guest Role
    // ? -----------------------------------------------------------------------

    guest_role_fetching_repo.get(guest_role_id).await
}
