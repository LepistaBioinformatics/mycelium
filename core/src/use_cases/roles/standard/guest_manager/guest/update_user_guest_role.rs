use crate::domain::{
    dtos::{guest::GuestUser, profile::Profile},
    entities::GuestUserOnAccountUpdating,
};

use clean_base::{
    entities::UpdatingResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use log::debug;
use uuid::Uuid;

/// Update the user's guest role.
///
/// This use case is used to replace the user's guest role. The user's guest
/// role is the role that the user has in the account.
///
pub async fn update_user_guest_role(
    profile: Profile,
    account_id: Uuid,
    old_guest_user_id: Uuid,
    new_guest_user_id: Uuid,
    guest_user_on_account_updating_repo: Box<&dyn GuestUserOnAccountUpdating>,
) -> Result<UpdatingResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    debug!("Requesting Profile: {:?}", profile);

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to update 
account guests."
                .to_string(),
        )
        .as_error();
    };

    guest_user_on_account_updating_repo
        .update(account_id, old_guest_user_id, new_guest_user_id)
        .await
}
