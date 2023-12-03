use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum DefaultActors {
    SubscriptionAccountManager,
    UserAccountManager,
    RoleManager,
    SystemManager,
}

impl Display for DefaultActors {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            DefaultActors::SubscriptionAccountManager => {
                write!(f, "subscription-account-manager")
            }
            DefaultActors::UserAccountManager => {
                write!(f, "user-account-manager")
            }
            DefaultActors::RoleManager => {
                write!(f, "role-manager")
            }
            DefaultActors::SystemManager => write!(f, "system-manager"),
        }
    }
}

impl FromStr for DefaultActors {
    type Err = ();

    fn from_str(s: &str) -> Result<DefaultActors, ()> {
        match s {
            "subscription-account-manager" => {
                Ok(DefaultActors::SubscriptionAccountManager)
            }
            "user-account-manager" => Ok(DefaultActors::UserAccountManager),
            "role-manager" => Ok(DefaultActors::RoleManager),
            "system-manager" => Ok(DefaultActors::SystemManager),

            _ => Err(()),
        }
    }
}
