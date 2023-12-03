use crate::{
    domain::{
        dtos::{profile::Profile, webhook::WebHook},
        entities::WebHookRegistration,
    },
    use_cases::roles::shared::webhook::default_actions::WebHookDefaultAction,
};

use clean_base::{entities::CreateResponseKind, utils::errors::MappedErrors};

pub async fn register_webhook(
    profile: Profile,
    url: String,
    action: WebHookDefaultAction,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
    profile.has_admin_privileges_or_error()?;
    webhook_registration_repo
        .create(action.as_webhook(url))
        .await
}
