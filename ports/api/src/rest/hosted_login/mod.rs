//! Gateway-hosted email login (OAuth 2.0 authorization code + PKCE).
//!
//! The client sends the browser to `authorize`, the gateway pages collect the
//! email and the emailed code, and the browser goes back to the client with a
//! one-time authorization code that the client exchanges at `token`.

mod authorize_url;
mod build_client_redirect;
mod exchange_token_url;
mod hosted_login_session_cookie;
mod render_hosted_login_page;
mod submit_code_url;
mod submit_email_url;

use actix_web::web;

/// Path prefix of every hosted login route, also the cookie path.
pub(crate) const HOSTED_LOGIN_PATH: &str = "/_adm/hosted-login";

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(authorize_url::authorize_url)
        .service(submit_email_url::submit_email_url)
        .service(submit_code_url::submit_code_url)
        .service(exchange_token_url::exchange_token_url);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ADMIN_API_SCOPE;

    #[test]
    fn path_matches_the_mounted_scope() {
        assert_eq!(
            HOSTED_LOGIN_PATH,
            format!("/{ADMIN_API_SCOPE}/hosted-login")
        );
    }
}
