use crate::domain::{
    actors::SystemActor,
    dtos::{
        http_secret::HttpSecret, native_error_codes::NativeErrorCodes,
        profile::Profile, webhook::WebHook,
    },
    entities::{WebHookFetching, WebHookUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_webhook",
    fields(profile_id = %profile.acc_id),
    skip_all,
)]
pub async fn update_webhook(
    profile: Profile,
    webhook_id: Uuid,
    name: Option<String>,
    description: Option<String>,
    secret: Option<HttpSecret>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
    webhook_updating_repo: Box<&dyn WebHookUpdating>,
) -> Result<UpdatingResponseKind<WebHook>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch webhook
    // ? -----------------------------------------------------------------------

    let mut webhook = match webhook_fetching_repo.get(webhook_id).await? {
        FetchResponseKind::Found(webhook) => webhook,
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "WebHook with id {} not found.",
                webhook_id
            ))
            .with_code(NativeErrorCodes::MYC00018)
            .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update webhook
    // ? -----------------------------------------------------------------------

    if let Some(name) = name {
        webhook.name = name;
    }

    if let Some(description) = description {
        webhook.description = Some(description);
    }

    if let Some(secret) = secret {
        webhook.set_secret(secret);
    }

    webhook_updating_repo.update(webhook).await
}
