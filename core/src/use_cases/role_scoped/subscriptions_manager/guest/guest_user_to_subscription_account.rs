use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::VerboseStatus, account_type::AccountType, email::Email,
            guest_user::GuestUser, native_error_codes::NativeErrorCodes,
            profile::Profile,
        },
        entities::{
            AccountFetching, GuestRoleFetching, GuestUserRegistration,
            MessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::send_email_notification,
};

use futures::future;
use mycelium_base::{
    dtos::Parent,
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Guest a user to perform actions into an account.
#[tracing::instrument(
    name = "guest_user_to_subscription_account",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn guest_user_to_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    email: Email,
    role_id: Uuid,
    target_account_id: Uuid,
    life_cycle_settings: AccountLifeCycle,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_read_write_access()
        .with_roles(vec![
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Guarantee needed information to evaluate guesting
    //
    // Check if the target account is a subscription account or a standard role
    // associated account. Only these accounts can receive guesting. Already
    // check the role_id to be a guest role is valid and exists.
    //
    // ? -----------------------------------------------------------------------

    let (target_account_response, target_role_response) = future::join(
        account_fetching_repo.get(target_account_id, related_accounts),
        guest_role_fetching_repo.get(role_id),
    )
    .await;

    let target_account = match target_account_response? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Target account not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00013)
            .as_error()
        }
        FetchResponseKind::Found(account) => match account.account_type {
            AccountType::Subscription { .. }
            | AccountType::RoleAssociated { .. }
            | AccountType::ActorAssociated { .. }
            | AccountType::TenantManager { .. } => account,
            _ => {
                return use_case_err(
                    "Invalid account. Only subscription accounts should \
                    receive guesting.",
                )
                .as_error()
            }
        },
    };

    if let Some(status) = target_account.verbose_status {
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

    let target_role = match target_role_response? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Guest role not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00013)
            .as_error()
        }
        FetchResponseKind::Found(role) => role,
    };

    // ? -----------------------------------------------------------------------
    // ? Persist changes
    // ? -----------------------------------------------------------------------

    let guest_user = match guest_user_registration_repo
        .get_or_create(
            GuestUser::new_unverified(
                email.to_owned(),
                Parent::Id(role_id),
                None,
            ),
            target_account_id,
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

    let mut parameters = vec![
        ("account_name", target_account.name.to_uppercase()),
        ("role_name", target_role.name.to_uppercase()),
        ("role_description", target_role.name.to_uppercase()),
        ("role_permissions", target_role.permission.to_string()),
    ];

    if let Some(description) = target_role.description {
        parameters.push(("role_description", description));
    }

    if let Err(err) = send_email_notification(
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
