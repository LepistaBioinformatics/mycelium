use crate::domain::dtos::hosted_login::generate_hosted_login_secret;

use serde::{Deserialize, Serialize};
use std::fmt;

const SUBJECT_REF_LENGTH: usize = 43;

/// The opaque reference a peer uses to name a linked user (D-5)
///
/// 32 random bytes, Base64URL without padding. Never an email and never a
/// raw user id, so a peer learns nothing from it and cannot enumerate users.
///
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SubjectRef(String);

impl SubjectRef {
    pub fn mint() -> Self {
        Self(generate_hosted_login_secret())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SubjectRef {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let well_formed = value.len() == SUBJECT_REF_LENGTH
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

        if !well_formed {
            return Err("malformed subject reference".to_string());
        }

        Ok(Self(value))
    }
}

impl From<SubjectRef> for String {
    fn from(value: SubjectRef) -> Self {
        value.0
    }
}

impl fmt::Display for SubjectRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minted_references_are_unique_and_parse_back() {
        let first = SubjectRef::mint();
        let second = SubjectRef::mint();

        assert_ne!(first, second);
        assert_eq!(SubjectRef::try_from(first.as_str().to_string()), Ok(first));
    }

    #[test]
    fn an_email_or_a_uuid_is_not_a_reference() {
        for value in [
            "user@example.com",
            "0b9f2d4e-7c1a-4d55-9a0e-3f6b8c2d1e7f",
            "",
        ] {
            assert!(SubjectRef::try_from(value.to_string()).is_err());
        }
    }

    #[test]
    fn json_carries_the_bare_string() {
        let reference = SubjectRef::mint();
        let json = serde_json::to_value(&reference).unwrap();

        assert_eq!(json, serde_json::json!(reference.as_str()));
        assert!(serde_json::from_value::<SubjectRef>(serde_json::json!("x"))
            .is_err());
    }
}
