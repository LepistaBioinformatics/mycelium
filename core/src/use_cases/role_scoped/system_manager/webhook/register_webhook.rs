use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            http::HttpMethod, 
            http_secret::HttpSecret, 
            profile::Profile, 
            written_by::WrittenBy, 
            webhook::{WebHook, WebHookTrigger}
        },
        entities::WebHookRegistration,
    }, 
    models::AccountLifeCycle
};

use mycelium_base::{
    entities::CreateResponseKind, utils::errors::{MappedErrors},
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
    method: Option<HttpMethod>,
    secret: Option<HttpSecret>,
    config: AccountLifeCycle,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::SystemManager])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Register webhook
    // ? -----------------------------------------------------------------------

    let webhook = WebHook::new_encrypted(
        name,
        description,
        url,
        trigger,
        method,
        secret,
        config,
        Some(WrittenBy::new_from_account(profile.acc_id)),
    )
    .await?;

    webhook_registration_repo
        .create(webhook)
        .await
}
