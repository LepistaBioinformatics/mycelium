use crate::{
    domain::{
        dtos::{
            account::VerboseStatus, account_type::AccountType, email::Email,
            guest_role::Permission, guest_user::GuestUser,
            native_error_codes::NativeErrorCodes, profile::Profile,
        },
        entities::{AccountFetching, GuestUserRegistration, LocalMessageWrite},
    },
    models::AccountLifeCycle,
    use_cases::support::dispatch_notification,
};

use mycelium_base::{
    dtos::Parent,
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "Guest user to subscription manager account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
        guest_email = %email.redacted_email(),
    ),
    skip(
        profile,
        email,
        life_cycle_settings,
        account_fetching_repo,
        guest_user_registration_repo,
        message_sending_repo,
    )
)]
pub async fn guest_user_to_subscription_manager_account(
    profile: Profile,
    email: Email,
    tenant_id: Uuid,
    permission: Permission,
    account_id: Uuid,
    life_cycle_settings: AccountLifeCycle,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
    let span: tracing::Span = tracing::Span::current();

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .get_tenant_wide_permission_or_error(tenant_id, Permission::Write)?;

    // ? -----------------------------------------------------------------------
    // ? Fetch account from data store
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo
        .get(account_id, related_accounts)
        .instrument(span.clone())
        .await?
    {
        FetchResponseKind::Found(account) => account,
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Account not found: {}", id.unwrap()))
                .with_code(NativeErrorCodes::MYC00018)
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the account is a subscription manager account
    // ? -----------------------------------------------------------------------

    let (read_role_id, write_role_id, role_name) =
        if let AccountType::RoleAssociated {
            tenant_id,
            read_role_id,
            write_role_id,
            role_name,
        } = account.account_type
        {
            if tenant_id != tenant_id {
                return use_case_err(format!(
                    "Account is not a subscription manager account: {}",
                    account_id
                ))
                .with_code(NativeErrorCodes::MYC00018)
                .as_error();
            }

            (read_role_id, write_role_id, role_name)
        } else {
            return use_case_err(
                "Account is not a subscription manager account: {}",
            )
            .with_code(NativeErrorCodes::MYC00018)
            .as_error();
        };

    // ? -----------------------------------------------------------------------
    // ? Check if the account is active
    // ? -----------------------------------------------------------------------

    if let Some(status) = account.verbose_status {
        if status != VerboseStatus::Verified {
            return use_case_err(
                "Invalid account status. Only active accounts should receive \
                guesting.",
            )
            .as_error();
        }
    } else {
        return use_case_err(
            "Unable to check account status for guesting. Account is maybe \
            inactive.",
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Create guest user
    // ? -----------------------------------------------------------------------

    let guest_role = match permission {
        Permission::Read => Parent::Id(read_role_id),
        Permission::Write => Parent::Id(write_role_id),
    };

    let guest_user = match guest_user_registration_repo
        .get_or_create(
            GuestUser::new_unverified(email.to_owned(), guest_role, None),
            account_id,
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            return use_case_err(format!("Unable to create guest user: {err}"))
                .with_code(NativeErrorCodes::MYC00017)
                .with_exp_true()
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    let parameters = vec![
        ("account_name", account.name.to_uppercase()),
        ("role_name", role_name.to_uppercase()),
        ("role_description", role_name.to_uppercase()),
        ("role_permissions", permission.to_string()),
    ];

    if let Err(err) = dispatch_notification(
        parameters,
        "email/guest-to-subscription-account",
        life_cycle_settings,
        email,
        None,
        message_sending_repo,
    )
    .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Send the guesting response
    // ? -----------------------------------------------------------------------

    Ok(guest_user)
}
