use super::invalid_verification_setting;
use crate::settings::PROFILE_JWKS_URL_ENV;

use mycelium_base::utils::errors::MappedErrors;
use std::fmt::{Display, Formatter, Result as FmtResult};
use url::Url;

/// Where a downstream verifier fetches the gateway profile JWKS
///
/// The location comes only from the service configuration, never from a
/// request: a verifier that followed a URL found in the message it verifies
/// would let an attacker choose the keys. `https://` is required; `http://`
/// is accepted only with the explicit development switch.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileJwksUrl(Url);

impl ProfileJwksUrl {
    pub fn parse(
        raw: &str,
        allow_insecure_http: bool,
    ) -> Result<Self, MappedErrors> {
        let url = Url::parse(raw.trim())
            .map_err(|_| invalid("is not an absolute URL"))?;

        if !url.username().is_empty() || url.password().is_some() {
            return Err(invalid("must not carry credentials"));
        }

        match url.scheme() {
            "https" => Ok(Self(url)),
            "http" if allow_insecure_http => {
                tracing::warn!(
                    event = "profile_signature.jwks",
                    "{PROFILE_JWKS_URL_ENV} uses plain HTTP; development only"
                );
                Ok(Self(url))
            }
            _ => Err(invalid("must use https")),
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for ProfileJwksUrl {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "{}", self.0)
    }
}

fn invalid(problem: &str) -> MappedErrors {
    invalid_verification_setting(PROFILE_JWKS_URL_ENV, problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTTPS: &str =
        "https://gateway.internal/.well-known/mycelium-profile-jwks.json";
    const HTTP: &str =
        "http://localhost:8080/.well-known/mycelium-profile-jwks.json";

    #[test]
    fn accepts_https() {
        let url = ProfileJwksUrl::parse(HTTPS, false).unwrap();
        assert_eq!(url.as_str(), HTTPS);
    }

    #[test]
    fn plain_http_needs_the_development_switch() {
        assert!(ProfileJwksUrl::parse(HTTP, false).is_err());
        assert!(ProfileJwksUrl::parse(HTTP, true).is_ok());
    }

    #[test]
    fn rejects_other_schemes_relative_urls_and_credentials() {
        for raw in [
            "",
            "/.well-known/mycelium-profile-jwks.json",
            "ftp://gateway/jwks.json",
            "file:///etc/jwks.json",
            "https://user:pass@gateway/jwks.json",
        ] {
            let error = ProfileJwksUrl::parse(raw, true).unwrap_err();
            assert!(error.has_str_code("MYC00035"), "{raw:?}");
            assert!(error.msg().contains("MYCELIUM_JWKS_URL"));
        }
    }
}
