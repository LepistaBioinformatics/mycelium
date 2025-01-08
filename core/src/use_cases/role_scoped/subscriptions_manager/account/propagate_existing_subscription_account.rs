use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::Account,
            profile::Profile,
            webhook::{WebHookPropagationResponse, WebHookTrigger},
        },
        entities::{AccountFetching, WebHookFetching},
    },
    models::AccountLifeCycle,
    use_cases::support::dispatch_webhooks,
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
    fields(profile_id = %profile.acc_id, target_account_id = %account_id),
    skip_all
)]
pub async fn propagate_existing_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    config: AccountLifeCycle,
    account_fetching_repo: Box<&dyn AccountFetching>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<WebHookPropagationResponse<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()?;

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

    let responses = dispatch_webhooks(
        WebHookTrigger::CreateSubscriptionAccount,
        account.to_owned(),
        config,
        webhook_fetching_repo,
    )
    .await;

    Ok(responses)
}
