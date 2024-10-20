use crate::{
    domain::{
        actors::ActorName,
        dtos::{profile::Profile, webhook::WebHook},
        entities::WebHookRegistration,
    },
    use_cases::roles::shared::webhook::default_actions::WebHookDefaultAction,
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "register_webhook",
    skip(profile, webhook_registration_repo)
)]
pub async fn register_webhook(
    profile: Profile,
    name: String,
    url: String,
    action: WebHookDefaultAction,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
    profile.get_default_create_ids_or_error(vec![
        ActorName::SystemManager.to_string()
    ])?;

    let mut webhook = action.as_webhook(url);
    webhook.name = name;

    webhook_registration_repo.create(webhook).await
}
