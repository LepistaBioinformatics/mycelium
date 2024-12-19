use crate::domain::{
    dtos::{
        guest_role::Permission, guest_user::GuestUser,
        native_error_codes::NativeErrorCodes, profile::Profile,
    },
    entities::GuestUserOnAccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Accept invitation to join an account
///
/// After a guest user has been invited to join an account, they can accept the
/// invitation to join the account.
///
#[tracing::instrument(name = "accept_invitation", skip_all)]
pub async fn accept_invitation(
    profile: Profile,
    account_id: Uuid,
    guest_role_id: Uuid,
    permission: Permission,
    guest_user_on_account_repo: Box<&dyn GuestUserOnAccountUpdating>,
) -> Result<UpdatingResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the profile licenses has the guest_user_id
    // ? -----------------------------------------------------------------------

    let licensed_resources = (match profile.licensed_resources {
        None => {
            return use_case_err("Profile does not have an account id")
                .with_exp_true()
                .as_error()
        }
        Some(licensed_resources) => licensed_resources,
    })
    .to_licenses_vector();

    let target_license = match licensed_resources
        .iter()
        .find(|license| license.guest_role_id == guest_role_id)
    {
        None => {
            return use_case_err("Profile does not have an account id")
                .with_exp_true()
                .as_error()
        }
        Some(license) => license,
    };

    if [
        target_license.acc_id == account_id,
        target_license.perm == permission,
        target_license.guest_role_id == guest_role_id,
        target_license.was_verified == false,
    ]
    .iter()
    .any(|&x| x == false)
    {
        return use_case_err(
            "Invalid operation. License does not match the invitation parameters",
        )
        .with_code(NativeErrorCodes::MYC00018)
        .with_exp_true()
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Accept invitation
    // ? -----------------------------------------------------------------------

    guest_user_on_account_repo
        .accept_invitation(guest_role_id, account_id, permission)
        .await
}
