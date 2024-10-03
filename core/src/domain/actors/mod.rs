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

    /// Subscription manager
    ///
    /// This actor is responsible for managing subscription accounts.
    SubscriptionManager,

    /// User account manager
    ///
    /// This actor is responsible for managing user accounts.
    UserManager,

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
            ActorName::SubscriptionManager => {
                write!(f, "subscription-manager")
            }
            ActorName::UserManager => {
                write!(f, "user-manager")
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
            "subscription-account-manager" => {
                Ok(ActorName::SubscriptionManager)
            }
            "subscription-manager" => Ok(ActorName::SubscriptionManager),
            "user-account-manager" => Ok(ActorName::UserManager),
            "user-manager" => Ok(ActorName::UserManager),
            "guest-manager" => Ok(ActorName::GuestManager),
            "system-manager" => Ok(ActorName::SystemManager),
            "tenant-manager" => Ok(ActorName::TenantManager),
            "tenant-owner" => Ok(ActorName::TenantOwner),

            _ => Err(()),
        }
    }
}
