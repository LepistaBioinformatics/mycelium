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
    GuestsManager,

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
    ///
    /// WARNING: This is not a role in the system. Don't use to filter licensed
    /// resource scopes during the profile checking.
    ///
    /// ❌ Wrong example:
    ///
    /// ```rust
    /// let related_accounts = profile
    ///     .on_tenant(tenant_id)
    ///     .with_system_accounts_access()
    ///     .with_write_access()
    ///     .with_roles(vec![
    ///         SystemActor::TenantOwner,
    ///         SystemActor::TenantManager,
    ///         SystemActor::SubscriptionsManager,
    ///     ])
    ///     .get_related_accounts_or_error()?;
    /// ```
    ///
    /// This way should check if the profile has access to the tenant as a guest
    /// role. However, tenant owner should be guest as a ownership not as a
    /// licensed resource.
    ///
    /// ✅ Right example:
    ///
    /// ```rust
    /// let related_accounts = profile
    ///     .on_tenant(tenant_id)
    ///     .with_system_accounts_access()
    ///     .with_write_access()
    ///     .with_roles(vec![
    ///         SystemActor::TenantManager,
    ///         SystemActor::SubscriptionsManager,
    ///     ])
    ///     .get_related_accounts_or_tenant_or_error(tenant_id)?;
    /// ```
    ///
    /// This way should check if the profile has ownership over the tenant.
    ///
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
            SystemActor::GuestsManager => {
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
            "subscriptions-account-manager" | "subscriptions-manager" => {
                Ok(SystemActor::SubscriptionsManager)
            }
            "users-account-manager" | "users-manager" => {
                Ok(SystemActor::UsersManager)
            }
            "account-manager" => Ok(SystemActor::AccountManager),
            "guest-manager" => Ok(SystemActor::GuestsManager),
            "gateway-manager" => Ok(SystemActor::GatewayManager),
            "system-manager" => Ok(SystemActor::SystemManager),
            "tenant-manager" => Ok(SystemActor::TenantManager),
            "tenant-owner" => Ok(SystemActor::TenantOwner),
            "service" => Ok(SystemActor::Service),

            other => {
                if other.starts_with("custom-role:") {
                    Ok(SystemActor::CustomRole(other[11..].to_string()))
                } else {
                    Err(())
                }
            }
        }
    }
}

impl SystemActor {
    pub fn str(&self) -> &str {
        match self {
            SystemActor::CustomRole(role) => role,
            SystemActor::Beginner => "beginner",
            SystemActor::SubscriptionsManager => "subscriptions-manager",
            SystemActor::UsersManager => "users-manager",
            SystemActor::AccountManager => "account-manager",
            SystemActor::GuestsManager => "guest-manager",
            SystemActor::GatewayManager => "gateway-manager",
            SystemActor::SystemManager => "system-manager",
            SystemActor::TenantOwner => "tenant-owner",
            SystemActor::TenantManager => "tenant-manager",
            SystemActor::Service => "service",
        }
    }
}
