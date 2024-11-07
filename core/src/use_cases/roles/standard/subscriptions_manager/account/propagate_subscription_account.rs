use crate::{
    domain::{
        actors::ActorName,
        dtos::{
            account::Account,
            profile::Profile,
            webhook::{AccountPropagationWebHookResponse, WebhookTrigger},
        },
        entities::WebHookFetching,
    },
    use_cases::support::dispatch_webhooks,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Propagate a new subscription account to all webhooks.
///
/// The propagation is done asynchronously, and the response is returned
/// immediately.
///
#[tracing::instrument(
    name = "propagate_subscription_account", 
    fields(profile_id = %profile.acc_id),
    skip(profile, bearer_token, account, webhook_fetching_repo)
)]
pub(super) async fn propagate_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    bearer_token: String,
    account: Account,
    trigger: WebhookTrigger,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_write_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionsManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Propagate new account
    // ? -----------------------------------------------------------------------

    let target_hooks =
        match webhook_fetching_repo.list_by_trigger(trigger).await? {
            FetchManyResponseKind::NotFound => None,
            FetchManyResponseKind::Found(records) => Some(records),
            FetchManyResponseKind::FoundPaginated(paginated_records) => {
                Some(paginated_records.records)
            }
        };

    let propagation_responses = match target_hooks {
        None => None,
        Some(hooks) => {
            dispatch_webhooks(hooks, account.to_owned(), Some(bearer_token))
                .await
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return created account
    // ? -----------------------------------------------------------------------

    Ok(AccountPropagationWebHookResponse {
        account,
        propagation_responses,
    })
}
