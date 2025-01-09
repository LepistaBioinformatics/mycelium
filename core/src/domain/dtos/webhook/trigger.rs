use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum WebHookTrigger {
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
    CreateUserAccount,

    /// Dispatched when a default user account is updated.
    UpdateUserAccount,

    /// Dispatched when a default user account is deleted.
    DeleteUserAccount,

    // ? -----------------------------------------------------------------------
    // ? Guesting related actions
    // ? -----------------------------------------------------------------------
    /// Dispatched when a guest account is created.
    InviteGuestAccount,

    /// Dispatched when a guest account is updated.
    UninviteGuestAccount,
}

impl Display for WebHookTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSubscriptionAccount => {
                write!(f, "createSubscriptionAccount")
            }
            Self::UpdateSubscriptionAccount => {
                write!(f, "updateSubscriptionAccount")
            }
            Self::DeleteSubscriptionAccount => {
                write!(f, "deleteSubscriptionAccount")
            }
            Self::CreateUserAccount => write!(f, "createUserAccount"),
            Self::UpdateUserAccount => write!(f, "updateUserAccount"),
            Self::DeleteUserAccount => write!(f, "deleteUserAccount"),
            Self::InviteGuestAccount => write!(f, "inviteGuestAccount"),
            Self::UninviteGuestAccount => write!(f, "uninviteGuestAccount"),
        }
    }
}

impl FromStr for WebHookTrigger {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CreateSubscriptionAccount" | "createSubscriptionAccount" => {
                Ok(Self::CreateSubscriptionAccount)
            }
            "UpdateSubscriptionAccount" | "updateSubscriptionAccount" => {
                Ok(Self::UpdateSubscriptionAccount)
            }
            "DeleteSubscriptionAccount" | "deleteSubscriptionAccount" => {
                Ok(Self::DeleteSubscriptionAccount)
            }
            "CreateUserAccount" | "createUserAccount" => {
                Ok(Self::CreateUserAccount)
            }
            "UpdateUserAccount" | "updateUserAccount" => {
                Ok(Self::UpdateUserAccount)
            }
            "DeleteUserAccount" | "deleteUserAccount" => {
                Ok(Self::DeleteUserAccount)
            }
            "InviteGuestAccount" | "inviteGuestAccount" => {
                Ok(Self::InviteGuestAccount)
            }
            "UninviteGuestAccount" | "uninviteGuestAccount" => {
                Ok(Self::UninviteGuestAccount)
            }
            _ => Err(format!("Unknown webhook trigger: {}", s)),
        }
    }
}
