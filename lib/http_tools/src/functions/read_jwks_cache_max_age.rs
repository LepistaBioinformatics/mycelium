use crate::settings::{
    PROFILE_JWKS_DEFAULT_MAX_AGE_SECONDS, PROFILE_JWKS_MAX_MAX_AGE_SECONDS,
    PROFILE_JWKS_MIN_MAX_AGE_SECONDS,
};

use std::time::Duration;

/// How long a fetched profile JWKS may be cached
///
/// Reads the `max-age` directive of a `Cache-Control` value and clamps it to
/// 1 minute..1 day; 12 hours when the header or the directive is absent or
/// unreadable. Shared rule of every SDK (see `jwks-vectors.json`).
///
pub fn read_jwks_cache_max_age(cache_control: Option<&str>) -> Duration {
    let announced = cache_control
        .and_then(|value| value.split(',').find_map(read_max_age_directive))
        .unwrap_or(PROFILE_JWKS_DEFAULT_MAX_AGE_SECONDS);

    Duration::from_secs(announced.clamp(
        PROFILE_JWKS_MIN_MAX_AGE_SECONDS,
        PROFILE_JWKS_MAX_MAX_AGE_SECONDS,
    ))
}

fn read_max_age_directive(directive: &str) -> Option<u64> {
    let (name, value) = directive.trim().split_once('=')?;

    name.trim()
        .eq_ignore_ascii_case("max-age")
        .then(|| value.trim().parse::<u64>().ok())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seconds(value: Option<&str>) -> u64 {
        read_jwks_cache_max_age(value).as_secs()
    }

    #[test]
    fn reads_and_clamps_max_age() {
        assert_eq!(seconds(Some("public, max-age=600")), 600);
        assert_eq!(seconds(Some("Max-Age=600")), 600);
        assert_eq!(seconds(Some("max-age=1")), 60);
        assert_eq!(seconds(Some("max-age=999999")), 86_400);
    }

    #[test]
    fn defaults_to_twelve_hours() {
        for value in [None, Some("no-cache"), Some("max-age=abc"), Some("")] {
            assert_eq!(seconds(value), 43_200, "{value:?}");
        }
    }
}
