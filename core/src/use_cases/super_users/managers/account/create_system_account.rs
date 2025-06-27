use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::{Account, Modifier},
        native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::AccountRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

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
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, account_registration_repo)
)]
pub async fn create_system_account(
    profile: Profile,
    name: String,
    actor: SystemActor,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Check if the desired actor should be created here
    // ? -----------------------------------------------------------------------

    let allowed_actors = [
        SystemActor::GatewayManager,
        SystemActor::GuestsManager,
        SystemActor::SystemManager,
    ];

    if !allowed_actors.contains(&actor) {
        return use_case_err(format!(
            "Only system actors accounts should be created. Given: {}",
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

    let mut unchecked_account = Account::new_actor_related_account(
        name,
        actor,
        true,
        Some(Modifier::new_from_account(profile.acc_id)),
    );

    unchecked_account.is_checked = true;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    account_registration_repo
        .get_or_create_actor_related_account(unchecked_account)
        .await
}
