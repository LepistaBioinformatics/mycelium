use actix_web::{http::StatusCode, HttpResponse};

/// Build the response for an HTML page rendered by the gateway itself.
///
/// Gateway pages carry login codes and claim forms, so no other origin may
/// frame them (clickjacking). Only framing is restricted: the templates rely
/// on inline styles and external fonts, which a stricter CSP would block.
pub(crate) fn hosted_page_response(
    status: StatusCode,
    html: String,
) -> HttpResponse {
    HttpResponse::build(status)
        .content_type("text/html; charset=utf-8")
        .insert_header(("X-Frame-Options", "DENY"))
        .insert_header(("Content-Security-Policy", "frame-ancestors 'none'"))
        .body(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_page_response_forbids_framing() {
        let response =
            hosted_page_response(StatusCode::OK, "<p>code</p>".to_string());
        let headers = response.headers();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(
            headers.get("Content-Security-Policy").unwrap(),
            "frame-ancestors 'none'"
        );
        assert_eq!(
            headers.get("Content-Type").unwrap(),
            "text/html; charset=utf-8"
        );
    }
}
