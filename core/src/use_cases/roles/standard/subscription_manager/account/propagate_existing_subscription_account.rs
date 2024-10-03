use super::propagate_subscription_account::propagate_subscription_account;
use crate::{
    domain::{
        actors::ActorName,
        dtos::{
            profile::Profile,
            webhook::{AccountPropagationWebHookResponse, HookTarget},
        },
        entities::{AccountFetching, WebHookFetching},
    },
    use_cases::roles::shared::webhook::default_actions::WebHookDefaultAction,
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
    fields(account_id = %profile.acc_id, target_account_id = %account_id),
    skip_all
)]
pub async fn propagate_existing_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    bearer_token: String,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_create_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
        ])?;

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

    propagate_subscription_account(
        profile,
        tenant_id,
        bearer_token,
        account,
        WebHookDefaultAction::CreateSubscriptionAccount,
        HookTarget::Account,
        webhook_fetching_repo,
    )
    .await
}
