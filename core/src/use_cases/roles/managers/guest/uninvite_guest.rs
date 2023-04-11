use crate::domain::{dtos::profile::Profile, entities::GuestUserDeletion};

use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use log::debug;
use uuid::Uuid;

/// Uninvite user to perform a role actions from account
///
pub async fn uninvite_guest(
    profile: Profile,
    account_id: Uuid,
    guest_user_id: Uuid,
    guest_user_deletion_repo: Box<&dyn GuestUserDeletion>,
) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    debug!("Requesting Profile: {:?}", profile);

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to uninvite accounts."
                .to_string(),
        ).as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Uninvite guest
    // ? -----------------------------------------------------------------------

    guest_user_deletion_repo
        .delete(guest_user_id, account_id)
        .await
}
