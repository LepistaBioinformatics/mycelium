use crate::domain::dtos::webhook::{HookTarget, WebHook};

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WebHookDefaultAction {
    // ? -----------------------------------------------------------------------
    // ? Subscription account related actions
    // ? -----------------------------------------------------------------------
    /// Dispatched when a subscription account is created.
    CreateSubscriptionAccount,

    /// Dispatched when a subscription account is updated.
    UpdateSubscriptionAccount,

    /// Dispatched when a subscription account is deleted.
    DeleteSubscriptionAccount,

    // ? -----------------------------------------------------------------------
    // ? Default user account related actions
    // ? -----------------------------------------------------------------------
    /// Dispatched when a default user account is created.
    CreateDefaultUserAccount,

    /// Dispatched when a default user account is updated.
    UpdateDefaultUserAccount,

    /// Dispatched when a default user account is deleted.
    DeleteDefaultUserAccount,

    // ? -----------------------------------------------------------------------
    // ? Guesting related actions
    // ? -----------------------------------------------------------------------
    /// Dispatched when a guest account is created.
    InviteGuestAccount,

    /// Dispatched when a guest account is updated.
    UninviteGuestAccount,
}

impl Display for WebHookDefaultAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebHookDefaultAction::CreateSubscriptionAccount => {
                write!(f, "createSubscriptionAccount")
            }
            WebHookDefaultAction::UpdateSubscriptionAccount => {
                write!(f, "updateSubscriptionAccount")
            }
            WebHookDefaultAction::DeleteSubscriptionAccount => {
                write!(f, "deleteSubscriptionAccount")
            }
            WebHookDefaultAction::CreateDefaultUserAccount => {
                write!(f, "createDefaultUserAccount")
            }
            WebHookDefaultAction::UpdateDefaultUserAccount => {
                write!(f, "updateDefaultUserAccount")
            }
            WebHookDefaultAction::DeleteDefaultUserAccount => {
                write!(f, "deleteDefaultUserAccount")
            }
            WebHookDefaultAction::InviteGuestAccount => {
                write!(f, "inviteGuestAccount")
            }
            WebHookDefaultAction::UninviteGuestAccount => {
                write!(f, "uninviteGuestAccount")
            }
        }
    }
}

impl WebHookDefaultAction {
    pub fn as_webhook(&self, url: String) -> Result<WebHook, ()> {
        match self {
            WebHookDefaultAction::CreateSubscriptionAccount => {
                Ok(WebHook::new(
                    self.to_string(),
                    "Dispatched when a subscription account is created."
                        .to_string(),
                    url,
                    HookTarget::Account,
                ))
            }
            WebHookDefaultAction::UpdateSubscriptionAccount => {
                Ok(WebHook::new(
                    self.to_string(),
                    "Dispatched when a subscription account is updated."
                        .to_string(),
                    url,
                    HookTarget::Account,
                ))
            }
            WebHookDefaultAction::DeleteSubscriptionAccount => {
                Ok(WebHook::new(
                    self.to_string(),
                    "Dispatched when a subscription account is deleted."
                        .to_string(),
                    url,
                    HookTarget::Account,
                ))
            }
            WebHookDefaultAction::CreateDefaultUserAccount => Ok(WebHook::new(
                self.to_string(),
                "Dispatched when a default user account is created."
                    .to_string(),
                url,
                HookTarget::Account,
            )),
            WebHookDefaultAction::UpdateDefaultUserAccount => Ok(WebHook::new(
                self.to_string(),
                "Dispatched when a default user account is updated."
                    .to_string(),
                url,
                HookTarget::Account,
            )),
            WebHookDefaultAction::DeleteDefaultUserAccount => Ok(WebHook::new(
                self.to_string(),
                "Dispatched when a default user account is deleted."
                    .to_string(),
                url,
                HookTarget::Account,
            )),
            WebHookDefaultAction::InviteGuestAccount => Ok(WebHook::new(
                self.to_string(),
                "Dispatched when a guest account is created.".to_string(),
                url,
                HookTarget::Guest,
            )),
            WebHookDefaultAction::UninviteGuestAccount => Ok(WebHook::new(
                self.to_string(),
                "Dispatched when a guest account is updated.".to_string(),
                url,
                HookTarget::Guest,
            )),
        }
    }
}
