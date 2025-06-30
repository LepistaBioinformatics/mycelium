use lazy_static::lazy_static;
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

/// Default email key
///
/// This is the default key used to store the email in the request headers and
/// send it to the gateway downstream services.
///
pub const DEFAULT_EMAIL_KEY: &str = "x-mycelium-email";

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

/// Default forwarded keys
///
/// This is the default key used to store the forwarded host, protocol and port
/// in the request headers and send it to the gateway downstream services.
///
pub const MYCELIUM_TARGET_HOST: &str = "x-mycelium-target-host";
pub const MYCELIUM_TARGET_PROTOCOL: &str = "x-mycelium-target-protocol";
pub const MYCELIUM_TARGET_PORT: &str = "x-mycelium-target-port";
pub const MYCELIUM_ROUTING_KEY: &str = "x-mycelium-routing";

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

/// Mycelium provider key
///
/// This is the key used to indicate that the request is coming from the
/// internal provider. This key should be used to validate the issuer of the
/// request.
///
pub const MYCELIUM_PROVIDER_KEY: &str = "mycelium";

// ? ---------------------------------------------------------------------------
// ? Authentication and authorization
// ? ---------------------------------------------------------------------------

lazy_static! {
    #[derive(Debug)]
    pub(crate) static ref PROFILE_FETCHING_URL: String =
        match var_os("PROFILE_FETCHING_URL") {
            Some(path) => path.into_string().unwrap(),
            None => panic!("PROFILE_FETCHING_URL not configured."),
        };
}
