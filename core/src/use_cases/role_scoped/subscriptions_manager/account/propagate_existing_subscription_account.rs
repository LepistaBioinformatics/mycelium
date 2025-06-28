use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::Account,
            profile::Profile,
            webhook::{PayloadId, WebHookTrigger},
        },
        entities::{AccountFetching, WebHookRegistration},
    },
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Propagate an existing subscription account to all webhooks.
///
/// The propagation is done asynchronously, and the response is returned
/// immediately.
///
#[tracing::instrument(
    name = "propagate_existing_subscription_account",
    fields(
        profile_id = %profile.acc_id,
        target_account_id = %account_id,
        correspondence_id = tracing::field::Empty,
    ),
    skip_all
)]
pub async fn propagate_existing_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
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

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_accounts_or_tenant_wide_permission_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Fetch subscription account
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo
        .get(account_id, related_accounts)
        .await?
    {
        FetchResponseKind::Found(account) => account,
        FetchResponseKind::NotFound(_) => {
            return use_case_err("The account was not found.".to_string())
                .as_error()
        }
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
    .await?;

    tracing::trace!("Side effects dispatched");

    Ok(account)
}
