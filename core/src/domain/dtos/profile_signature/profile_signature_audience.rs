use crate::domain::dtos::native_error_codes::NativeErrorCodes;

use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};

/// Downstream service a profile signature was issued for
///
/// The `aud` claim. The gateway fills it with the name of the service the
/// request is routed to, so a downstream that configures its expected
/// audience rejects a signed pair replayed from another service.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProfileSignatureAudience(String);

impl ProfileSignatureAudience {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProfileSignatureAudience {
    type Error = MappedErrors;

    fn try_from(service_name: String) -> Result<Self, Self::Error> {
        if service_name.trim().is_empty() {
            return dto_err("profile_signature_audience_is_empty")
                .with_code(NativeErrorCodes::MYC00034)
                .as_error();
        }

        Ok(Self(service_name))
    }
}

impl From<ProfileSignatureAudience> for String {
    fn from(audience: ProfileSignatureAudience) -> Self {
        audience.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_service_name_verbatim() {
        let audience =
            ProfileSignatureAudience::try_from("billing-api".to_string())
                .unwrap();

        assert_eq!(audience.as_str(), "billing-api");
    }

    #[test]
    fn rejects_blank_names() {
        for blank in ["", " ", "\t"] {
            assert!(
                ProfileSignatureAudience::try_from(blank.to_string()).is_err()
            );
        }
    }
}
