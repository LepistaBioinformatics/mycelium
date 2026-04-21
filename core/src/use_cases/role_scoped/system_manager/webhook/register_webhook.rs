use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            http::HttpMethod,
            http_secret::HttpSecret,
            profile::Profile,
            webhook::{WebHook, WebHookTrigger},
            written_by::WrittenBy,
        },
        entities::{EncryptionKeyFetching, WebHookRegistration},
        utils::{build_aad, AAD_FIELD_HTTP_SECRET},
    },
    models::AccountLifeCycle,
};

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
    method: Option<HttpMethod>,
    secret: Option<HttpSecret>,
    config: AccountLifeCycle,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    encryption_key_fetching_repo: Box<&dyn EncryptionKeyFetching>,
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
    // ? Fetch the system DEK (webhooks are global, no tenant)
    // ? -----------------------------------------------------------------------

    let kek = config.derive_kek_bytes().await?;
    let dek = encryption_key_fetching_repo
        .get_or_provision_dek(None, &kek)
        .await?;

    let aad = build_aad(None, AAD_FIELD_HTTP_SECRET);

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
        &dek,
        &aad,
        Some(WrittenBy::new_from_account(profile.acc_id)),
    )?;

    webhook_registration_repo.create(webhook).await
}
