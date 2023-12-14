use crate::domain::{
    actors::DefaultActor,
    dtos::{
        profile::Profile,
        webhook::{HookTarget, WebHook},
    },
    entities::WebHookFetching,
};

use clean_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

pub async fn list_webhooks(
    profile: Profile,
    name: Option<String>,
    target: Option<HookTarget>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
    profile
        .get_view_ids_or_error(vec![DefaultActor::SystemManager.to_string()])?;

    webhook_fetching_repo.list(name, target).await
}
