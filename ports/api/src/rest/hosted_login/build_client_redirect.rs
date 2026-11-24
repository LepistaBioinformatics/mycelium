use url::Url;

/// `redirect_uri` with `code` and `state` appended to its query
/// (RFC 6749 §4.1.2). `None` if the registered URI is not a valid URL.
pub(super) fn build_client_redirect(
    redirect_uri: &str,
    authorization_code: &str,
    state: &str,
) -> Option<String> {
    let mut url = Url::parse(redirect_uri).ok()?;

    url.query_pairs_mut()
        .append_pair("code", authorization_code)
        .append_pair("state", state);

    Some(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_code_and_state() {
        let redirect = build_client_redirect(
            "https://app.example.com/callback",
            "abc",
            "x y&z",
        );

        assert_eq!(
            redirect.as_deref(),
            Some("https://app.example.com/callback?code=abc&state=x+y%26z")
        );
    }

    #[test]
    fn keeps_existing_query() {
        let redirect = build_client_redirect(
            "https://app.example.com/callback?tenant=1",
            "abc",
            "s",
        );

        assert_eq!(
            redirect.as_deref(),
            Some("https://app.example.com/callback?tenant=1&code=abc&state=s")
        );
    }

    #[test]
    fn rejects_invalid_uri() {
        assert!(build_client_redirect("not a url", "abc", "s").is_none());
    }
}
