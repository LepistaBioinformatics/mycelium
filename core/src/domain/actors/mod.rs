use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ActorName {
    CustomRole(String),

    /// No role
    ///
    /// This actor is used when no role is assigned to the user.
    NoRole,

    /// Subscriptions manager
    ///
    /// This actor is responsible for managing subscriptions accounts.
    SubscriptionsManager,

    /// Users account manager
    ///
    /// This actor is responsible for managing users accounts.
    UsersManager,

    /// Account manager
    ///
    /// This actor is responsible for managing a single subscription account.
    AccountManager,

    /// Guest manager
    ///
    /// This actor is responsible for managing roles, guest-roles, and
    /// guest-users.
    GuestManager,

    /// System manager
    ///
    /// This actor is responsible for managing system, including error messages,
    /// webhooks, and others.
    SystemManager,

    /// Tenant owner
    ///
    /// This actor is responsible for managing tenant metadata, tags, and owner.
    TenantOwner,

    /// Tenant manager
    ///
    /// This actor is responsible for managing tenants.
    TenantManager,
}

impl Display for ActorName {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            ActorName::CustomRole(role) => write!(f, "custom-role:{}", role),
            ActorName::NoRole => write!(f, "no-role"),
            ActorName::SubscriptionsManager => {
                write!(f, "subscriptions-manager")
            }
            ActorName::UsersManager => {
                write!(f, "users-manager")
            }
            ActorName::AccountManager => {
                write!(f, "account-manager")
            }
            ActorName::GuestManager => {
                write!(f, "guest-manager")
            }
            ActorName::SystemManager => write!(f, "system-manager"),
            ActorName::TenantOwner => write!(f, "tenant-owner"),
            ActorName::TenantManager => write!(f, "tenant-manager"),
        }
    }
}

impl FromStr for ActorName {
    type Err = ();

    fn from_str(s: &str) -> Result<ActorName, ()> {
        match s {
            "no-role" => Ok(ActorName::NoRole),
            "subscriptions-account-manager" => {
                Ok(ActorName::SubscriptionsManager)
            }
            "subscriptions-manager" => Ok(ActorName::SubscriptionsManager),
            "users-account-manager" => Ok(ActorName::UsersManager),
            "users-manager" => Ok(ActorName::UsersManager),
            "account-manager" => Ok(ActorName::AccountManager),
            "guest-manager" => Ok(ActorName::GuestManager),
            "system-manager" => Ok(ActorName::SystemManager),
            "tenant-manager" => Ok(ActorName::TenantManager),
            "tenant-owner" => Ok(ActorName::TenantOwner),

            _ => Err(()),
        }
    }
}
