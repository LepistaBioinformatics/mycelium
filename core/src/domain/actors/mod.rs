use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;

/// The System Actors
///
/// Standard actors used to validate operations during the authorization process
/// in system use-cases.
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SystemActor {
    CustomRole(String),

    /// Beginner
    ///
    /// This actor is used when no role is assigned to the user.
    Beginner,

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

    /// Gateway manager
    ///
    /// This actor is responsible for managing gateway endpoints and related
    /// configurations.
    GatewayManager,

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

    /// Service
    ///
    /// This is a service entity.
    Service,
}

impl Display for SystemActor {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            SystemActor::CustomRole(role) => write!(f, "custom-role:{}", role),
            SystemActor::Beginner => write!(f, "beginners"),
            SystemActor::SubscriptionsManager => {
                write!(f, "subscriptions-manager")
            }
            SystemActor::UsersManager => {
                write!(f, "users-manager")
            }
            SystemActor::AccountManager => {
                write!(f, "accounts-manager")
            }
            SystemActor::GuestManager => {
                write!(f, "guests-manager")
            }
            SystemActor::GatewayManager => {
                write!(f, "gateway-manager")
            }
            SystemActor::SystemManager => write!(f, "system-manager"),
            SystemActor::TenantOwner => write!(f, "tenant-owner"),
            SystemActor::TenantManager => write!(f, "tenant-manager"),
            SystemActor::Service => write!(f, "service"),
        }
    }
}

impl FromStr for SystemActor {
    type Err = ();

    fn from_str(s: &str) -> Result<SystemActor, ()> {
        match s {
            "beginner" | "no-role" => Ok(SystemActor::Beginner),
            "subscriptions-account-manager" => {
                Ok(SystemActor::SubscriptionsManager)
            }
            "subscriptions-manager" => Ok(SystemActor::SubscriptionsManager),
            "users-account-manager" => Ok(SystemActor::UsersManager),
            "users-manager" => Ok(SystemActor::UsersManager),
            "account-manager" => Ok(SystemActor::AccountManager),
            "guest-manager" => Ok(SystemActor::GuestManager),
            "gateway-manager" => Ok(SystemActor::GatewayManager),
            "system-manager" => Ok(SystemActor::SystemManager),
            "tenant-manager" => Ok(SystemActor::TenantManager),
            "tenant-owner" => Ok(SystemActor::TenantOwner),
            "service" => Ok(SystemActor::Service),

            _ => Err(()),
        }
    }
}
