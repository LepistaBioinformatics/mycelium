use crate::domain::{
    actors::SystemActor,
    dtos::{
        profile::Profile,
        webhook::{WebHook, WebHookTrigger},
    },
    entities::WebHookFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "list_webhooks",
    skip(profile, webhook_fetching_repo)
)]
pub async fn list_webhooks(
    profile: Profile,
    name: Option<String>,
    trigger: Option<WebHookTrigger>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch webhooks
    // ? -----------------------------------------------------------------------

    webhook_fetching_repo.list(name, trigger).await
}
