use actix_web::http::header::{HeaderMap, HeaderName};
use myc_http_tools::settings::MYCELIUM_HEADER_PREFIX;

/// Strip client-supplied Mycelium headers from the downstream request
///
/// ! Trust boundary
///
/// `awc::Client::request_from` copies every header of the client request into
/// the downstream request. Headers under the `x-mycelium-` namespace carry
/// identity context that downstream services attribute to the gateway — a
/// client that forges one would be trusted by the SDKs as if the gateway had
/// authenticated it.
///
/// Everything under the namespace is therefore removed unconditionally, before
/// the gateway injects its own values. Deny by default, re-grant explicitly:
/// whatever the gateway legitimately forwards is re-inserted by the caller
/// after this runs. Do not turn this into a list of known keys — a new
/// `x-mycelium-*` header would silently fall outside it.
///
#[tracing::instrument(name = "strip_inbound_mycelium_headers", skip_all)]
pub(super) fn strip_inbound_mycelium_headers(headers: &mut HeaderMap) {
    let inbound_mycelium_headers = headers
        .keys()
        .filter(|name| name.as_str().starts_with(MYCELIUM_HEADER_PREFIX))
        .cloned()
        .collect::<Vec<HeaderName>>();

    for header_name in inbound_mycelium_headers {
        tracing::debug!(
            stage = "router.strip_inbound_mycelium_headers",
            header = header_name.as_str(),
            "Client-supplied Mycelium header removed"
        );

        headers.remove(&header_name);
    }
}

// ? ---------------------------------------------------------------------------
// ? Tests
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::strip_inbound_mycelium_headers;

    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
    use myc_http_tools::settings::{
        DEFAULT_CONNECTION_STRING_KEY, DEFAULT_EMAIL_KEY,
        DEFAULT_MYCELIUM_ROLE_KEY, DEFAULT_PROFILE_KEY, DEFAULT_SCOPE_KEY,
        DEFAULT_TENANT_ID_KEY,
    };

    fn headers_from(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();

        for (name, value) in pairs {
            headers.append(
                HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        headers
    }

    #[test]
    fn forged_profile_is_removed() {
        let mut headers =
            headers_from(&[(DEFAULT_PROFILE_KEY, "forged-profile")]);

        strip_inbound_mycelium_headers(&mut headers);

        assert!(headers.get(DEFAULT_PROFILE_KEY).is_none());
    }

    #[test]
    fn headers_without_a_producer_are_removed() {
        let mut headers = headers_from(&[
            (DEFAULT_SCOPE_KEY, "forged-scope"),
            (DEFAULT_MYCELIUM_ROLE_KEY, "forged-role"),
            (DEFAULT_TENANT_ID_KEY, "forged-tenant"),
            (DEFAULT_EMAIL_KEY, "forged@email.com"),
        ]);

        strip_inbound_mycelium_headers(&mut headers);

        assert!(headers.get(DEFAULT_SCOPE_KEY).is_none());
        assert!(headers.get(DEFAULT_MYCELIUM_ROLE_KEY).is_none());
        assert!(headers.get(DEFAULT_TENANT_ID_KEY).is_none());
        assert!(headers.get(DEFAULT_EMAIL_KEY).is_none());
    }

    #[test]
    fn connection_string_is_not_forwarded_downstream() {
        let mut headers =
            headers_from(&[(DEFAULT_CONNECTION_STRING_KEY, "user-credential")]);

        strip_inbound_mycelium_headers(&mut headers);

        assert!(headers.get(DEFAULT_CONNECTION_STRING_KEY).is_none());
    }

    #[test]
    fn non_mycelium_headers_are_untouched() {
        let mut headers = headers_from(&[
            ("authorization", "Bearer token"),
            ("content-type", "application/json"),
            ("x-crab-shell-session", "session-42"),
        ]);

        strip_inbound_mycelium_headers(&mut headers);

        assert_eq!(headers.get("authorization").unwrap(), "Bearer token");

        assert_eq!(headers.get("content-type").unwrap(), "application/json");

        assert_eq!(headers.get("x-crab-shell-session").unwrap(), "session-42");
    }

    #[test]
    fn matching_is_prefix_based_and_case_insensitive() {
        let mut headers = headers_from(&[
            ("X-Mycelium-Profile", "forged-profile"),
            ("x-mycelium", "not-namespaced"),
            ("x-my-header", "unrelated"),
        ]);

        strip_inbound_mycelium_headers(&mut headers);

        assert!(headers.get("x-mycelium-profile").is_none());
        assert_eq!(headers.get("x-mycelium").unwrap(), "not-namespaced");
        assert_eq!(headers.get("x-my-header").unwrap(), "unrelated");
    }

    #[test]
    fn multi_valued_headers_keep_every_value() {
        let mut headers = headers_from(&[
            ("accept", "text/html"),
            ("accept", "application/json"),
            (DEFAULT_PROFILE_KEY, "forged-profile"),
        ]);

        strip_inbound_mycelium_headers(&mut headers);

        let accepted = headers
            .get_all("accept")
            .map(|value| value.to_str().unwrap().to_owned())
            .collect::<Vec<String>>();

        assert_eq!(accepted, vec!["text/html", "application/json"]);
        assert!(headers.get(DEFAULT_PROFILE_KEY).is_none());
    }
}
