use crate::{
    models::{FetchedProfileJwks, ProfileJwksUrl, TrustedProfileKeys},
    settings::{PROFILE_JWKS_FETCH_TIMEOUT_SECONDS, PROFILE_JWKS_MAX_BYTES},
};

use super::read_jwks_cache_max_age;

use actix_web::{
    http::header::{HeaderMap, ACCEPT, CACHE_CONTROL},
    web::Bytes,
};
use awc::Client;
use futures::future::LocalBoxFuture;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use std::time::Duration;

/// Download and parse the gateway profile JWKS over HTTP
///
/// Bounded: 5 s timeout, 64 KiB body, redirects never followed (a redirect
/// could move the trust anchor to another origin). The cache lifetime comes
/// from `Cache-Control: max-age`, clamped to 1 minute..1 day, 12 h when
/// absent. Needs the actix runtime (any actix-web service has it).
///
pub fn fetch_profile_jwks(
    url: ProfileJwksUrl,
) -> LocalBoxFuture<'static, Result<FetchedProfileJwks, MappedErrors>> {
    Box::pin(fetch(url))
}

#[tracing::instrument(name = "fetch_profile_jwks", skip_all)]
async fn fetch(
    url: ProfileJwksUrl,
) -> Result<FetchedProfileJwks, MappedErrors> {
    let (cache_control, body) = download(&url).await?;

    Ok(FetchedProfileJwks {
        keys: TrustedProfileKeys::from_jwks_document(&body)?,
        max_age: read_jwks_cache_max_age(cache_control.as_deref()),
    })
}

/// The `Cache-Control` value and the bounded body of a successful response
async fn download(
    url: &ProfileJwksUrl,
) -> Result<(Option<String>, Bytes), MappedErrors> {
    let mut response = client()
        .get(url.as_str())
        .insert_header((ACCEPT, "application/json"))
        .send()
        .await
        .map_err(|error| failure(&format!("request failed: {error}")))?;

    if !response.status().is_success() {
        return Err(failure(&format!("status {}", response.status())));
    }

    let cache_control = cache_control_of(response.headers());
    let body = response
        .body()
        .limit(PROFILE_JWKS_MAX_BYTES)
        .await
        .map_err(|error| failure(&format!("body unreadable: {error}")))?;

    Ok((cache_control, body))
}

fn cache_control_of(headers: &HeaderMap) -> Option<String> {
    headers
        .get(CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(PROFILE_JWKS_FETCH_TIMEOUT_SECONDS))
        .disable_redirects()
        .finish()
}

fn failure(problem: &str) -> MappedErrors {
    execution_err(format!("profile_jwks_fetch_failed: {problem}"))
}
