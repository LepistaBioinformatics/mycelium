use crate::domain::{
    actors::ActorName,
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

    profile.get_default_read_ids_or_error(vec![ActorName::SystemManager])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch webhooks
    // ? -----------------------------------------------------------------------

    webhook_fetching_repo.list(name, trigger).await
}
