use crate::{
    domain::{
        actors::SystemActor::*,
        dtos::{
            email::Email, guest_role::Permission, guest_user::GuestUser,
            native_error_codes::NativeErrorCodes,
            token::RoleScopedConnectionString,
        },
        entities::{
            AccountRegistration, GuestRoleFetching, GuestUserRegistration,
            LocalMessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::{
        get_or_create_role_related_account, send_email_notification,
    },
};

use mycelium_base::{
    dtos::Parent,
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::{info, warn};
use uuid::Uuid;

/// Guest a user to a default account
///
/// This method should be called from webhooks to propagate a new user to a
/// default account.
#[tracing::instrument(name = "guest_to_default_account", skip_all)]
pub async fn guest_to_default_account(
    scope: RoleScopedConnectionString,
    role_id: Uuid,
    email: Email,
    tenant_id: Uuid,
    life_cycle_settings: AccountLifeCycle,
    account_registration_repo: Box<&dyn AccountRegistration>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    message_sending_repo: Box<&dyn LocalMessageSending>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check permissions
    // ? -----------------------------------------------------------------------

    scope.contain_tenant_enough_permissions(
        tenant_id,
        role_id,
        vec![
            (GuestsManager.to_string(), Permission::Write),
            (SubscriptionsManager.to_string(), Permission::Write),
        ],
    )?;

    // ? -----------------------------------------------------------------------
    // ? Guarantee needed information to evaluate guesting
    //
    // Check if the target account is a subscription account or a standard role
    // associated account. Only these accounts can receive guesting. Already
    // check the role_id to be a guest role is valid and exists.
    //
    // ? -----------------------------------------------------------------------

    let target_role = match guest_role_fetching_repo.get(role_id).await? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Guest role not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00012)
            .as_error()
        }
        FetchResponseKind::Found(role) => role,
    };

    let default_subscription_account = match get_or_create_role_related_account(
        Some(target_role.name.to_owned()),
        tenant_id,
        role_id,
        None,
        account_registration_repo,
    )
    .await?
    {
        GetOrCreateResponseKind::NotCreated(account, _) => account,
        GetOrCreateResponseKind::Created(account) => account,
    };

    // ? -----------------------------------------------------------------------
    // ? Persist changes
    // ? -----------------------------------------------------------------------

    match guest_user_registration_repo
        .get_or_create(
            GuestUser::new_unverified(
                email.to_owned(),
                Parent::Id(role_id),
                None,
            ),
            match default_subscription_account.id {
                None => {
                    warn!(
                        "Default account maybe invalid. ID not found: {:?}",
                        default_subscription_account
                    );

                    return use_case_err("Invalid default account".to_string())
                        .as_error();
                }
                Some(id) => id,
            },
        )
        .await?
    {
        GetOrCreateResponseKind::Created(guest_user) => {
            info!("Guest user created: {}", guest_user.email.redacted_email());
        }
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(format!("Guest user not created: {msg}"))
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    let mut parameters = vec![
        (
            "account_name",
            default_subscription_account.name.to_uppercase(),
        ),
        ("role_name", target_role.name.to_uppercase()),
        ("role_description", target_role.name),
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

    Ok(())
}
