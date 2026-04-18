use crate::domain::{dtos::profile::Profile, entities::TokenDeletion};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};

/// Revoke a connection string (set expiration to now)
///
/// Marks the token as expired so it is immediately invalid but remains
/// visible in the listing with the "Expired" badge.
///
#[tracing::instrument(
    name = "revoke_connection_string",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn revoke_connection_string(
    profile: Profile,
    token_id: u32,
    token_deletion_repo: Box<&dyn TokenDeletion>,
) -> Result<DeletionResponseKind<u32>, MappedErrors> {
    token_deletion_repo
        .revoke_connection_string(profile.acc_id, token_id)
        .await
}
