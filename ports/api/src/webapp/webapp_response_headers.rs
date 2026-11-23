use actix_web::{
    http::header::{self, HeaderValue},
    HttpResponse,
};

const NO_STORE: &str = "no-store, no-cache, must-revalidate";

const IMMUTABLE: &str = "public, max-age=31536000, immutable";

const IMMUTABLE_EXTENSIONS: [&str; 10] = [
    "js", "css", "woff", "woff2", "png", "svg", "ico", "webp", "jpg", "jpeg",
];

/// The cache policy of the webapp's `nginx.conf` for one file (WEBAPP-07)
///
/// `index.html` and the service worker are never cached, so a new build is
/// picked up on the next load; the hashed assets are immutable.
///
pub(super) fn cache_control_for(file_name: &str) -> Option<&'static str> {
    if file_name == "index.html" || file_name == "sw.js" {
        return Some(NO_STORE);
    }

    let (_, extension) = file_name.rsplit_once('.')?;
    let extension = extension.to_ascii_lowercase();

    IMMUTABLE_EXTENSIONS
        .contains(&extension.as_str())
        .then_some(IMMUTABLE)
}

/// The security headers of every `/_app/` answer, plus the cache policy of
/// the file served, if any
pub(super) fn with_webapp_headers(
    mut response: HttpResponse,
    file_name: Option<&str>,
) -> HttpResponse {
    let headers = response.headers_mut();

    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("SAMEORIGIN"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    if let Some(policy) = file_name.and_then(cache_control_for) {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(policy));
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_caches_the_index_nor_the_service_worker() {
        assert_eq!(cache_control_for("index.html"), Some(NO_STORE));
        assert_eq!(cache_control_for("sw.js"), Some(NO_STORE));
    }

    #[test]
    fn caches_hashed_assets_for_good() {
        for name in ["index-AbC1.js", "index-AbC1.css", "font.woff2", "a.SVG"] {
            assert_eq!(cache_control_for(name), Some(IMMUTABLE), "{name}");
        }
    }

    #[test]
    fn leaves_other_files_to_the_defaults() {
        for name in ["manifest.webmanifest", "robots.txt", "LICENSE"] {
            assert_eq!(cache_control_for(name), None, "{name}");
        }
    }
}
