use lazy_static::lazy_static;
use reqwest::Client;
use std::env::var_os;

// ? ---------------------------------------------------------------------------
// ? Configure default system constants
// ? ---------------------------------------------------------------------------

/// Default profile key
///
/// This is the default key used to store the profile in the request headers and
/// send it to the gateway downstream services.
///
pub const DEFAULT_PROFILE_KEY: &str = "x-mycelium-profile";

/// Default scope key
///
/// The scope key should be used to inject the scope present on the connection
/// string into the request headers and send it to the gateway downstream
/// services.
///
pub const DEFAULT_SCOPE_KEY: &str = "x-mycelium-scope";

/// Default mycelium role key
///
/// This is the default key used to store the mycelium role in the request
/// headers and send it to the gateway downstream services.
///
pub const DEFAULT_MYCELIUM_ROLE_KEY: &str = "x-mycelium-role";

/// Default request id key
///
/// This is the default key used to store the request id in the request headers
/// and send it to the gateway downstream services.
///
pub const DEFAULT_REQUEST_ID_KEY: &str = "x-mycelium-request-id";

/// Default connection string key
///
/// This is the default key used to store the connection string in the request
/// headers and send it to the gateway downstream services.
///
pub const DEFAULT_CONNECTION_STRING_KEY: &str = "x-mycelium-connection-string";

/// Default tenant id key
///
/// This is the default key used to store the tenant id in the request headers
/// and send it to the gateway downstream services.
///
pub const DEFAULT_TENANT_ID_KEY: &str = "x-mycelium-tenant-id";

/// Default forward header key
///
/// This is the default key used to store the forward header in the request
/// headers and send it to the gateway downstream services.
///
pub const FORWARD_FOR_KEY: &str = "x-forwarded-for";

/// Default forwarding keys
///
/// Such keys are used to map the headers that should be removed from the
/// downstream response before stream it back to the client.
///
pub const FORWARDING_KEYS: [&str; 9] = [
    "Host",
    "Connection",
    "Keep-Alive",
    "Proxy-Authenticate",
    "Proxy-Authorization",
    "Te",
    "Trailers",
    "Transfer-Encoding",
    "Upgrade",
];

// ? ---------------------------------------------------------------------------
// ? Authentication and authorization
// ? ---------------------------------------------------------------------------

lazy_static! {
    #[derive(Debug)]
    pub(super) static ref REQWEST_CLIENT: Client = Client::new();
}

pub(super) async fn get_client() -> Client {
    REQWEST_CLIENT.to_owned()
}

lazy_static! {

    #[derive(Debug)]
    pub(crate) static ref PROFILE_FETCHING_URL: String =
        match var_os("PROFILE_FETCHING_URL") {
            Some(path) => path.into_string().unwrap(),
            None => panic!("PROFILE_FETCHING_URL not configured."),
        };
}
