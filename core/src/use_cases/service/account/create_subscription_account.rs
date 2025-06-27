use crate::{
    domain::{
        dtos::{
            account::{Account, Modifier},
            native_error_codes::NativeErrorCodes,
            token::UserAccountConnectionString,
            webhook::{PayloadId, WebHookTrigger},
        },
        entities::{AccountRegistration, WebHookRegistration},
    },
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Create an account flagged as subscription.
///
/// Subscription accounts represents results centering accounts.
#[tracing::instrument(
    name = "create_subscription_account",
    fields(
        owner_id = %connection_string.account_id,
        correspondence_id = tracing::field::Empty
    ),
    skip(
        connection_string,
        account_registration_repo,
        webhook_registration_repo
    )
)]
pub async fn create_subscription_account(
    connection_string: UserAccountConnectionString,
    tenant_id: Uuid,
    account_name: String,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<Account, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", &Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a subscription account");

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    //connection_string.contain_enough_permissions(
    //    tenant_id,
    //    vec![
    //        (TenantManager.to_string(), Permission::Write),
    //        (SubscriptionsManager.to_string(), Permission::Write),
    //    ],
    //)?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account = Account::new_subscription_account(
        account_name,
        tenant_id,
        Some(Modifier::new_from_account(connection_string.account_id)),
    );

    unchecked_account.is_checked = true;

    let account = match account_registration_repo
        .create_subscription_account(unchecked_account, tenant_id)
        .await?
    {
        CreateResponseKind::NotCreated(account, msg) => {
            return use_case_err(format!("({}): {}", account.name, msg))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        }
        CreateResponseKind::Created(account) => account,
    };

    // ? -----------------------------------------------------------------------
    // ? Register the webhook
    // ? -----------------------------------------------------------------------

    tracing::trace!("Dispatching side effects");

    let account_id = account.id.ok_or_else(|| {
        use_case_err("Account ID not found".to_string()).with_exp_true()
    })?;

    register_webhook_dispatching_event(
        correspondence_id,
        WebHookTrigger::SubscriptionAccountCreated,
        account.to_owned(),
        PayloadId::Uuid(account_id),
        webhook_registration_repo,
    )
    .await?;

    tracing::trace!("Side effects dispatched");

    Ok(account)
}
