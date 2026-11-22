use super::PreparedFederation;

use myc_core::domain::dtos::hosted_login::HostedLoginClient;
use myc_http_tools::settings::FEDERATION_LINK_CALLBACK_PATH;

/// The hosted login client the issuer's linking pages sign in through
pub(crate) const LINK_LOGIN_CLIENT_ID: &str = "mycelium-federation-link";

/// Where the hosted login sends a linking user back on this instance
pub(crate) fn link_callback_url(prepared: &PreparedFederation) -> String {
    format!(
        "{}{FEDERATION_LINK_CALLBACK_PATH}",
        prepared.config.base_url.trim_end_matches('/')
    )
}

/// The internal client, accepted by the hosted login only while
/// `[core.federation]` is present; its single redirect URI is this
/// instance's own callback, so codes never leave the gateway
pub(crate) fn link_login_client(
    prepared: &PreparedFederation,
) -> HostedLoginClient {
    HostedLoginClient {
        client_id: LINK_LOGIN_CLIENT_ID.to_string(),
        redirect_uris: vec![link_callback_url(prepared)],
    }
}
