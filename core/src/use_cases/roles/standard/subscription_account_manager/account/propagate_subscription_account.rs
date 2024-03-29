use crate::{
    domain::{
        actors::DefaultActor,
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

/// Propagate a new subscription account to all webhooks.
///
/// The propagation is done asynchronously, and the response is returned
/// immediately.
///
pub(super) async fn propagate_subscription_account(
    profile: Profile,
    bearer_token: String,
    account: Account,
    webhook_default_action: WebHookDefaultAction,
    hook_target: HookTarget,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_create_ids_or_error(vec![
        DefaultActor::SubscriptionAccountManager.to_string(),
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
