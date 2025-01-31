use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(
    Clone, Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash,
)]
pub enum WebHookTrigger {
    // ? -----------------------------------------------------------------------
    // ? Subscription account related actions
    // ? -----------------------------------------------------------------------
    #[serde(rename = "subscriptionAccount.created")]
    SubscriptionAccountCreated,
    //#[serde(rename = "subscriptionAccount.updated")]
    //SubscriptionAccountUpdated,
    //#[serde(rename = "subscriptionAccount.deleted")]
    //SubscriptionAccountDeleted,

    // ? -----------------------------------------------------------------------
    // ? Default user account related actions
    // ? -----------------------------------------------------------------------
    #[serde(rename = "userAccount.created")]
    UserAccountCreated,
    //#[serde(rename = "userAccount.updated")]
    //UserAccountUpdated,
    //#[serde(rename = "userAccount.deleted")]
    //UserAccountDeleted,
}

impl Display for WebHookTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SubscriptionAccountCreated => {
                write!(f, "subscriptionAccount.created")
            }
            //Self::SubscriptionAccountUpdated => {
            //    write!(f, "subscriptionAccount.updated")
            //}
            //Self::SubscriptionAccountDeleted => {
            //    write!(f, "subscriptionAccount.deleted")
            //}
            Self::UserAccountCreated => write!(f, "userAccount.created"),
            //Self::UserAccountUpdated => write!(f, "userAccount.updated"),
            //Self::UserAccountDeleted => write!(f, "userAccount.deleted"),
        }
    }
}

impl FromStr for WebHookTrigger {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "subscriptionAccount.created" => {
                Ok(Self::SubscriptionAccountCreated)
            }
            //"subscriptionAccount.updated" => {
            //    Ok(Self::SubscriptionAccountUpdated)
            //}
            //"subscriptionAccount.deleted" => {
            //    Ok(Self::SubscriptionAccountDeleted)
            //}
            "userAccount.created" => Ok(Self::UserAccountCreated),
            //"userAccount.updated" => Ok(Self::UserAccountUpdated),
            //"userAccount.deleted" => Ok(Self::UserAccountDeleted),
            _ => Err(format!("Unknown webhook trigger: {}", s)),
        }
    }
}
