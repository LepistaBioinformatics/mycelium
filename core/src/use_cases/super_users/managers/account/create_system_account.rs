use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::Account, native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::{AccountRegistration, GuestRoleFetching},
};

use mycelium_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::error;
use uuid::Uuid;

/// Create a system account
///
/// System accounts should be used to guest users to system roles. Only system
/// accounts. System accounts are privileged accounts destined to group users
/// with specific roles, such as:
///
/// - Guest Managers
/// - System Managers
/// - Gateway Managers
///
#[tracing::instrument(
    name = "create_system_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, guest_role_fetching_repo, account_registration_repo)
)]
pub async fn create_system_account(
    profile: Profile,
    account_name: String,
    tenant_id: Uuid,
    role: SystemActor,
    guest_role_id: Uuid,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch guest role
    // ? -----------------------------------------------------------------------

    let guest_role = match guest_role_fetching_repo.get(guest_role_id).await? {
        FetchResponseKind::Found(role) => role,
        FetchResponseKind::NotFound(msg) => {
            error!(
                "Guest role not found: {msg}",
                msg = match msg {
                    Some(msg) => msg.to_string(),
                    None => "No message provided".to_string(),
                }
            );

            return use_case_err("Guest role not found")
                .with_code(NativeErrorCodes::MYC00018)
                .with_exp_true()
                .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the desired actor should be created here
    // ? -----------------------------------------------------------------------

    let allowed_actors = [
        SystemActor::GatewayManager,
        SystemActor::GuestManager,
        SystemActor::GatewayManager,
    ];

    if !allowed_actors.contains(&role) {
        return use_case_err(format!(
            "Only system actors are allowed to be created here. Given: {}",
            allowed_actors
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .with_code(NativeErrorCodes::MYC00013)
        .with_exp_true()
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Create and register account
    // ? -----------------------------------------------------------------------

    let mut unchecked_account = Account::new_role_related_account(
        account_name,
        tenant_id,
        guest_role_id,
        role,
    );

    unchecked_account.is_checked = true;
    unchecked_account.is_default = true;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    account_registration_repo
        .get_or_create_role_related_account(unchecked_account)
        .await
}
