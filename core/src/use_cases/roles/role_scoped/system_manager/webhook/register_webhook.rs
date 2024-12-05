use crate::{domain::{
    actors::ActorName,
    dtos::{
        profile::Profile,
        webhook::{WebHook, WebHookSecret, WebHookTrigger},
    },
    entities::WebHookRegistration,
}, models::AccountLifeCycle};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};

#[tracing::instrument(
    name = "register_webhook", 
    fields(profile_id = %profile.acc_id), 
    skip_all
)]
pub async fn register_webhook(
    profile: Profile,
    name: String,
    description: Option<String>,
    url: String,
    trigger: WebHookTrigger,
    secret: Option<WebHookSecret>,
    config: AccountLifeCycle,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![ActorName::SystemManager])?;

    // ? -----------------------------------------------------------------------
    // ? Register webhook
    // ? -----------------------------------------------------------------------

    let webhook = WebHook::new_encrypted(name, description, url, trigger, secret, config)?;

    webhook_registration_repo
        .create(webhook)
        .await
}
