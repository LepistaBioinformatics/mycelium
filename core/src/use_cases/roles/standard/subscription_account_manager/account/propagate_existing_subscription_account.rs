use super::propagate_subscription_account::propagate_subscription_account;
use crate::{
    domain::{
        actors::DefaultActor,
        dtos::{
            profile::Profile,
            webhook::{AccountPropagationWebHookResponse, HookTarget},
        },
        entities::{AccountFetching, WebHookFetching},
    },
    use_cases::roles::shared::webhook::default_actions::WebHookDefaultAction,
};

use clean_base::{
    entities::FetchResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Propagate an existing subscription account to all webhooks.
///
/// The propagation is done asynchronously, and the response is returned
/// immediately.
///
pub async fn propagate_existing_subscription_account(
    profile: Profile,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_create_ids_or_error(vec![
        DefaultActor::SubscriptionAccountManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch subscription account
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo.get(account_id).await? {
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
        account,
        WebHookDefaultAction::CreateSubscriptionAccount,
        HookTarget::Account,
        webhook_fetching_repo,
    )
    .await
}
