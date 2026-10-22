use super::HOSTED_LOGIN_PATH;

use actix_web::{
    cookie::{Cookie, SameSite},
    HttpRequest,
};

const HOSTED_LOGIN_COOKIE: &str = "myc_hosted_login";

/// Cookie binding the browser to its hosted login session. `Secure` follows
/// the public URL scheme so local `http://` setups keep working.
pub(super) fn hosted_login_session_cookie(
    session_id: &str,
    secure: bool,
) -> Cookie<'static> {
    Cookie::build(HOSTED_LOGIN_COOKIE, session_id.to_owned())
        .path(HOSTED_LOGIN_PATH)
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .finish()
}

pub(super) fn read_hosted_login_session_id(
    req: &HttpRequest,
) -> Option<String> {
    req.cookie(HOSTED_LOGIN_COOKIE)
        .map(|cookie| cookie.value().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_is_http_only_lax_and_scoped_to_hosted_login() {
        let cookie = hosted_login_session_cookie("session", true);

        assert_eq!(cookie.value(), "session");
        assert_eq!(cookie.path(), Some(HOSTED_LOGIN_PATH));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    }

    #[test]
    fn cookie_is_not_secure_on_plain_http() {
        let cookie = hosted_login_session_cookie("session", false);

        assert_eq!(cookie.secure(), Some(false));
    }
}
