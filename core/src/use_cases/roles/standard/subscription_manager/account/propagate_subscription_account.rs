use crate::{
    domain::{
        actors::ActorName,
        dtos::{
            account::Account,
            profile::Profile,
            webhook::{AccountPropagationWebHookResponse, HookTarget},
        },
        entities::WebHookFetching,
    },
    use_cases::roles::shared::webhook::{
        default_actions::WebHookDefaultAction, dispatch_webhooks,
    },
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
    fields(account_id = %profile.acc_id, hook_target = %hook_target),
    skip_all
)]
pub(super) async fn propagate_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    bearer_token: String,
    account: Account,
    webhook_default_action: WebHookDefaultAction,
    hook_target: HookTarget,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_create_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Propagate new account
    // ? -----------------------------------------------------------------------

    let target_hooks = match webhook_fetching_repo
        .list(Some(webhook_default_action.to_string()), Some(hook_target))
        .await?
    {
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
