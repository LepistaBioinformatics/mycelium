use crate::domain::{
    dtos::profile::Profile, entities::TokenDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};

/// Hard-delete a connection string
///
/// Permanently removes the token row. The token will no longer appear in
/// the listing.
///
#[tracing::instrument(
    name = "delete_connection_string",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn delete_connection_string(
    profile: Profile,
    token_id: u32,
    token_deletion_repo: Box<&dyn TokenDeletion>,
) -> Result<DeletionResponseKind<u32>, MappedErrors> {
    token_deletion_repo
        .delete_connection_string(profile.acc_id, token_id)
        .await
}
