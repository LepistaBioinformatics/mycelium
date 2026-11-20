use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A capability granted by a scoped (notification) connection string
///
/// A connection string carrying any scope is not a general credential: it is
/// accepted only by the endpoint that requires that scope, and rejected on
/// every surface that resolves a profile.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, ToSchema,
)]
pub enum NotificationScope {
    /// Send email notifications on behalf of a service
    #[serde(rename = "notify:email")]
    NotifyEmail,
}

impl ToString for NotificationScope {
    fn to_string(&self) -> String {
        match self {
            NotificationScope::NotifyEmail => "notify:email".to_string(),
        }
    }
}

impl TryFrom<&str> for NotificationScope {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "notify:email" => Ok(NotificationScope::NotifyEmail),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_round_trip() {
        let scope = NotificationScope::NotifyEmail;

        assert_eq!(scope.to_string(), "notify:email");
        assert_eq!(
            NotificationScope::try_from(scope.to_string().as_str()),
            Ok(scope)
        );
        assert!(NotificationScope::try_from("notify:sms").is_err());
    }

    #[test]
    fn json_round_trip() {
        let json =
            serde_json::to_string(&NotificationScope::NotifyEmail).unwrap();

        assert_eq!(json, "\"notify:email\"");
        assert_eq!(
            serde_json::from_str::<NotificationScope>(&json).unwrap(),
            NotificationScope::NotifyEmail
        );
    }
}
