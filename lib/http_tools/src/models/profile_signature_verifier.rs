use super::{
    invalid_verification_setting, ProfileSignatureTolerance, TrustedProfileKeys,
};
use crate::settings::{
    PROFILE_SIGNATURE_AUDIENCE_ENV, PROFILE_SIGNATURE_TOLERANCE_ENV,
};

use myc_core::domain::dtos::profile_signature::ProfileSignatureAudience;
use mycelium_base::utils::errors::MappedErrors;
use std::{collections::BTreeMap, env::vars_os};

/// Settings of a downstream profile signature verifier
///
/// Verification is opt-in per service: enabled when at least one
/// `MYC_PROFILE_PUBLIC_KEY_<version>` variable exists. An invalid setting is
/// always an error, never "disabled".
///
#[derive(Clone, Debug)]
pub struct ProfileSignatureVerifier {
    keys: TrustedProfileKeys,
    tolerance: ProfileSignatureTolerance,
    expected_audience: Option<ProfileSignatureAudience>,
}

impl ProfileSignatureVerifier {
    pub fn new(
        keys: TrustedProfileKeys,
        tolerance: ProfileSignatureTolerance,
        expected_audience: Option<ProfileSignatureAudience>,
    ) -> Self {
        Self {
            keys,
            tolerance,
            expected_audience,
        }
    }

    /// Build the verifier from environment-like variables
    ///
    /// Returns `None` when no public key variable exists.
    ///
    pub fn from_variables(
        variables: &BTreeMap<String, String>,
    ) -> Result<Option<Self>, MappedErrors> {
        let tolerance = ProfileSignatureTolerance::from_variable(
            variables.get(PROFILE_SIGNATURE_TOLERANCE_ENV),
        )?;
        let expected_audience =
            parse_audience(variables.get(PROFILE_SIGNATURE_AUDIENCE_ENV))?;

        let Some(keys) = TrustedProfileKeys::from_variables(variables)? else {
            return Ok(None);
        };

        Ok(Some(Self::new(keys, tolerance, expected_audience)))
    }

    /// Build the verifier from the process environment
    ///
    /// A variable whose value is not valid UTF-8 is read as empty, so it
    /// fails validation instead of silently disappearing.
    ///
    pub fn from_environment() -> Result<Option<Self>, MappedErrors> {
        let variables = vars_os()
            .filter_map(|(name, value)| {
                let name = name.into_string().ok()?;
                Some((name, value.into_string().unwrap_or_default()))
            })
            .collect();

        Self::from_variables(&variables)
    }

    pub fn keys(&self) -> &TrustedProfileKeys {
        &self.keys
    }

    pub fn tolerance(&self) -> ProfileSignatureTolerance {
        self.tolerance
    }

    pub fn expected_audience(&self) -> Option<&ProfileSignatureAudience> {
        self.expected_audience.as_ref()
    }
}

fn parse_audience(
    raw: Option<&String>,
) -> Result<Option<ProfileSignatureAudience>, MappedErrors> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    ProfileSignatureAudience::try_from(raw.to_owned())
        .map(Some)
        .map_err(|_| {
            invalid_verification_setting(
                PROFILE_SIGNATURE_AUDIENCE_ENV,
                "must not be empty",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TEST ONLY: RFC 8032 test vector 1 (SPKI)
    const PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=
-----END PUBLIC KEY-----
";

    fn variables(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn disabled_without_public_keys() {
        let verifier =
            ProfileSignatureVerifier::from_variables(&variables(&[])).unwrap();

        assert!(verifier.is_none());
    }

    #[test]
    fn reads_tolerance_and_audience() {
        let verifier = ProfileSignatureVerifier::from_variables(&variables(&[
            ("MYC_PROFILE_PUBLIC_KEY_1", PUBLIC_PEM),
            ("MYC_PROFILE_SIGNATURE_TOLERANCE_SECONDS", "30"),
            ("MYC_PROFILE_SIGNATURE_AUDIENCE", "billing-api"),
        ]))
        .unwrap()
        .unwrap();

        assert_eq!(verifier.tolerance().seconds(), 30);
        assert_eq!(
            verifier.expected_audience().map(|aud| aud.as_str()),
            Some("billing-api")
        );
    }

    #[test]
    fn invalid_settings_fail_even_without_keys() {
        for entry in [
            ("MYC_PROFILE_SIGNATURE_AUDIENCE", "  "),
            ("MYC_PROFILE_SIGNATURE_TOLERANCE_SECONDS", "0"),
        ] {
            let error =
                ProfileSignatureVerifier::from_variables(&variables(&[entry]))
                    .unwrap_err();

            assert!(error.has_str_code("MYC00035"));
        }
    }
}
