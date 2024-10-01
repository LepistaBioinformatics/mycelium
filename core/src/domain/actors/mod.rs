use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum DefaultActor {
    /// Subscription account manager
    ///
    /// This actor is responsible for managing subscription accounts.
    NoRole,

    /// Subscription account manager
    ///
    /// This actor is responsible for managing subscription accounts.
    SubscriptionAccountManager,

    /// User account manager
    ///
    /// This actor is responsible for managing user accounts.
    UserAccountManager,

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

impl Display for DefaultActor {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            DefaultActor::NoRole => write!(f, "no-role"),
            DefaultActor::SubscriptionAccountManager => {
                write!(f, "subscription-account-manager")
            }
            DefaultActor::UserAccountManager => {
                write!(f, "user-account-manager")
            }
            DefaultActor::GuestManager => {
                write!(f, "guest-manager")
            }
            DefaultActor::SystemManager => write!(f, "system-manager"),
            DefaultActor::TenantOwner => write!(f, "tenant-owner"),
            DefaultActor::TenantManager => write!(f, "tenant-manager"),
        }
    }
}

impl FromStr for DefaultActor {
    type Err = ();

    fn from_str(s: &str) -> Result<DefaultActor, ()> {
        match s {
            "no-role" => Ok(DefaultActor::NoRole),
            "subscription-account-manager" => {
                Ok(DefaultActor::SubscriptionAccountManager)
            }
            "user-account-manager" => Ok(DefaultActor::UserAccountManager),
            "guest-manager" => Ok(DefaultActor::GuestManager),
            "system-manager" => Ok(DefaultActor::SystemManager),
            "tenant-manager" => Ok(DefaultActor::TenantManager),
            "tenant-owner" => Ok(DefaultActor::TenantOwner),

            _ => Err(()),
        }
    }
}
