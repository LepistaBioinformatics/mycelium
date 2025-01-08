use crate::domain::{
    actors::SystemActor,
    dtos::{
        native_error_codes::NativeErrorCodes, profile::Profile,
        webhook::WebHook,
    },
    entities::WebHookUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_webhook",
    skip(profile, webhook_updating_repo)
)]
pub async fn update_webhook(
    profile: Profile,
    webhook: WebHook,
    webhook_id: Uuid,
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
    // ? Update webhook
    // ? -----------------------------------------------------------------------

    let target_webhook_id = match webhook.id.to_owned() {
        Some(id) => id,
        None => {
            return use_case_err(
                "WebHook id is required to update a WebHook.".to_string(),
            )
            .with_code(NativeErrorCodes::MYC00018)
            .as_error()
        }
    };

    if webhook_id != target_webhook_id {
        return use_case_err(
            "WebHook id does not match the path id.".to_string(),
        )
        .with_code(NativeErrorCodes::MYC00018)
        .as_error();
    };

    webhook_updating_repo.update(webhook).await
}
