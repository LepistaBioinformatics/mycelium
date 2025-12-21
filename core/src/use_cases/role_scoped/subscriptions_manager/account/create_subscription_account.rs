use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::Account,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            written_by::WrittenBy,
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
use tracing::Instrument;
use uuid::Uuid;

/// Create an account flagged as subscription.
///
/// Subscription accounts represents results centering accounts.
#[tracing::instrument(
    name = "create_subscription_account",
    fields(
        profile_id = %profile.acc_id,
        correspondence_id = tracing::field::Empty,
    ),
    skip_all
)]
pub async fn create_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_name: String,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<Account, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    let span = tracing::Span::current();
    span.record("correspondence_id", Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a subscription account");

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let is_owner = profile.with_tenant_ownership_or_error(tenant_id).is_ok();

    let has_access = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()
        .is_ok();

    if ![is_owner, has_access].iter().any(|&x| x) {
        return use_case_err(
            "Insufficient privileges to create a subscription account",
        )
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account = Account::new_subscription_account(
        account_name,
        tenant_id,
        Some(WrittenBy::new_from_account(profile.acc_id)),
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
    // ? Propagate account
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
    .instrument(span)
    .await?;

    tracing::trace!("Side effects dispatched");

    Ok(account)
}
