use super::invalid_verification_setting;
use crate::settings::{PROFILE_JWKS_PATH, PROFILE_JWKS_URL_ENV};

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
/// Two forms are accepted, so a service only needs to know where its gateway
/// is:
///
/// - the complete endpoint, ending in `PROFILE_JWKS_PATH`, used as is;
/// - the gateway base (`mycelium.example.com`, `https://host/prefix`), to
///   which `PROFILE_JWKS_PATH` is appended. Without a scheme, `https://` is
///   assumed.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileJwksUrl(Url);

impl ProfileJwksUrl {
    pub fn parse(
        raw: &str,
        allow_insecure_http: bool,
    ) -> Result<Self, MappedErrors> {
        let url = Url::parse(&with_default_scheme(raw.trim())?)
            .map_err(|_| invalid("is not an absolute URL"))?;

        if !url.username().is_empty() || url.password().is_some() {
            return Err(invalid("must not carry credentials"));
        }

        check_scheme(&url, allow_insecure_http)?;
        with_jwks_path(url).map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// The configured value, with `https://` when it names no scheme
fn with_default_scheme(configured: &str) -> Result<String, MappedErrors> {
    if configured.contains("://") {
        return Ok(configured.to_string());
    }

    if configured.is_empty() || configured.starts_with('/') {
        return Err(invalid("is not an absolute URL"));
    }

    Ok(format!("https://{configured}"))
}

fn check_scheme(
    url: &Url,
    allow_insecure_http: bool,
) -> Result<(), MappedErrors> {
    match url.scheme() {
        "https" => Ok(()),
        "http" if allow_insecure_http => {
            tracing::warn!(
                event = "profile_signature.jwks",
                "{PROFILE_JWKS_URL_ENV} uses plain HTTP; development only"
            );
            Ok(())
        }
        _ => Err(invalid("must use https")),
    }
}

/// The complete endpoint as is, or the gateway base plus the JWKS path
fn with_jwks_path(url: Url) -> Result<Url, MappedErrors> {
    if url.path().ends_with(PROFILE_JWKS_PATH) {
        return Ok(url);
    }

    if url.query().is_some() || url.fragment().is_some() {
        return Err(invalid("must not carry a query or fragment"));
    }

    let base = url.as_str().trim_end_matches('/');

    Url::parse(&format!("{base}{PROFILE_JWKS_PATH}"))
        .map_err(|_| invalid("is not an absolute URL"))
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
    fn gateway_base_gets_the_jwks_path() {
        for (configured, resolved) in [
            ("gateway.internal", HTTPS),
            ("https://gateway.internal", HTTPS),
            ("https://gateway.internal/", HTTPS),
            (
                "gateway.internal/prod",
                "https://gateway.internal/prod/.well-known/mycelium-profile-jwks.json",
            ),
        ] {
            let url = ProfileJwksUrl::parse(configured, false).unwrap();
            assert_eq!(url.as_str(), resolved, "{configured:?}");
        }
    }

    #[test]
    fn rejects_other_schemes_relative_urls_and_credentials() {
        for raw in [
            "",
            "/.well-known/mycelium-profile-jwks.json",
            "ftp://gateway/jwks.json",
            "file:///etc/jwks.json",
            "https://user:pass@gateway/jwks.json",
            "https://gateway.internal/prod?tenant=1",
        ] {
            let error = ProfileJwksUrl::parse(raw, true).unwrap_err();
            assert!(error.has_str_code("MYC00043"), "{raw:?}");
            assert!(error.msg().contains("MYCELIUM_JWKS_URL"));
        }
    }
}
