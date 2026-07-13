use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum IDSource {
    /// The ID source is the user ID
    User,

    /// The ID source is the system actor
    Account,
}

impl IDSource {
    fn marker_prefix(&self) -> &'static str {
        match self {
            IDSource::User => "user",
            IDSource::Account => "account",
        }
    }
}

/// Identifies who performed a write. `id`/`from` are `None` whenever no
/// User/Account exists yet at write time (e.g. the staff bootstrap flow,
/// where only the operator's typed email is known before any account is
/// created) -- `email` alone still carries useful information in that case.
#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct WrittenBy {
    /// The ID of the user or account who performed the action.
    #[serde(default)]
    pub id: Option<Uuid>,

    /// The source `id` belongs to. `None` whenever `id` is `None`.
    #[serde(default)]
    pub from: Option<IDSource>,

    /// Base64-encoded email of the actor, when known.
    #[serde(default)]
    pub email: Option<String>,
}

impl WrittenBy {
    fn new(
        id: Option<Uuid>,
        from: Option<IDSource>,
        email: Option<String>,
    ) -> Self {
        Self { id, from, email }
    }

    pub fn new_from_user(id: Uuid) -> Self {
        Self::new(Some(id), Some(IDSource::User), None)
    }

    pub fn new_from_account(id: Uuid) -> Self {
        Self::new(Some(id), Some(IDSource::Account), None)
    }

    pub fn new_from_user_with_email(id: Uuid, email: &str) -> Self {
        Self::new(
            Some(id),
            Some(IDSource::User),
            Some(Self::encode_email(email)),
        )
    }

    pub fn new_from_account_with_email(id: Uuid, email: &str) -> Self {
        Self::new(
            Some(id),
            Some(IDSource::Account),
            Some(Self::encode_email(email)),
        )
    }

    /// Identity known only by email -- no User/Account exists yet.
    pub fn new_from_email(email: &str) -> Self {
        Self::new(None, None, Some(Self::encode_email(email)))
    }

    /// Create a new written by with no information at all.
    pub fn new_anemic() -> Self {
        Self::new(None, None, None)
    }

    fn encode_email(email: &str) -> String {
        general_purpose::STANDARD.encode(email.as_bytes())
    }
}

impl Default for WrittenBy {
    fn default() -> Self {
        Self::new_anemic()
    }
}

/// Renders a compact, informative marker for logs/audit trails, e.g.
/// `user:<uuid>`, `account:<uuid>;email:<base64>`, or `email:<base64>` alone
/// when no id/from is known yet.
impl fmt::Display for WrittenBy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let identity = match (&self.id, &self.from) {
            (Some(id), Some(from)) => format!("{}:{id}", from.marker_prefix()),
            _ => String::new(),
        };

        let email = self
            .email
            .as_ref()
            .map(|value| format!("email:{value}"))
            .unwrap_or_default();

        match (identity.is_empty(), email.is_empty()) {
            (false, false) => write!(f, "{identity};{email}"),
            (false, true) => write!(f, "{identity}"),
            (true, false) => write!(f, "{email}"),
            (true, true) => write!(f, "unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let updated_by = WrittenBy::default();
        assert_eq!(updated_by.id, None);
        assert_eq!(updated_by.from, None);
        assert_eq!(updated_by.email, None);
        assert_eq!(updated_by.to_string(), "unknown");
    }

    #[test]
    fn marker_includes_source_and_id() {
        let id = Uuid::new_v4();
        let written_by = WrittenBy::new_from_account(id);
        assert_eq!(written_by.to_string(), format!("account:{id}"));
    }

    #[test]
    fn marker_combines_identity_and_email() {
        let id = Uuid::new_v4();
        let written_by =
            WrittenBy::new_from_user_with_email(id, "staff@example.com");
        let expected_email =
            general_purpose::STANDARD.encode("staff@example.com");
        assert_eq!(
            written_by.to_string(),
            format!("user:{id};email:{expected_email}")
        );
    }

    #[test]
    fn marker_is_email_only_when_no_id_exists_yet() {
        let written_by = WrittenBy::new_from_email("staff@example.com");
        let expected_email =
            general_purpose::STANDARD.encode("staff@example.com");
        assert_eq!(written_by.to_string(), format!("email:{expected_email}"));
        assert_eq!(written_by.id, None);
        assert_eq!(written_by.from, None);
    }

    #[test]
    fn deserializes_pre_existing_records_without_an_email_field() {
        let legacy_json = serde_json::json!({
            "id": Uuid::new_v4(),
            "from": "user",
        });

        let written_by: WrittenBy =
            serde_json::from_value(legacy_json).unwrap();

        assert!(written_by.id.is_some());
        assert_eq!(written_by.from, Some(IDSource::User));
        assert_eq!(written_by.email, None);
    }
}
