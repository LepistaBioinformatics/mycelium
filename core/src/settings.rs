use crate::{
    domain::dtos::route::Route,
    use_cases::gateway::routes::load_config_from_yaml,
};

use futures::lock::Mutex;
use lazy_static::lazy_static;
use std::env::var_os;
use tera::Tera;

// ? ---------------------------------------------------------------------------
// ? Configure routes and profile
//
// Here default system constants are configured.
// ? ---------------------------------------------------------------------------

/// Default profile key
///
/// This is the default key used to store the profile in the request headers and
/// send it to the gateway downstream services.
///
pub const DEFAULT_PROFILE_KEY: &str = "x-mycelium-profile";

/// Default service account key
///
/// This is the default key used to store the service account in the request
/// headers and send it to the gateway downstream services.
///
pub const DEFAULT_SERVICE_ACCOUNT_KEY: &str = "x-mycelium-sa-token";

/// Default TOTP domain
///
/// This is the default domain used to generate the TOTP token.
///
pub const DEFAULT_TOTP_DOMAIN: &str = "Mycelium";

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
// ? Configure routes and profile
//
// Here routes and profile services are loaded.
// ? ---------------------------------------------------------------------------

lazy_static! {
    pub static ref ROUTES: Mutex<Vec<Route>> = Mutex::new(vec![]);
}

pub async fn init_in_memory_routes(routes_file: Option<String>) {
    let source_file_path = match routes_file {
        None => {
            match var_os("SOURCE_FILE_PATH") {
                Some(path) => Some(path.into_string().unwrap()),
                None => {
                    panic!("Required environment variable SOURCE_FILE_PATH not set.")
                }
            }
        }
        Some(path) => Some(path),
    };

    let db = match load_config_from_yaml(match source_file_path.to_owned() {
        None => panic!(
            "Source path not already loaded. Please run the init method before 
                load database."
        ),
        Some(path) => path,
    })
    .await
    {
        Err(err) => {
            panic!("Unexpected error on load in memory database: {err}")
        }
        Ok(res) => res,
    };

    ROUTES.lock().await.extend(db);
}

// ? ---------------------------------------------------------------------------
// ? Templates
// ? ---------------------------------------------------------------------------

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut _tera = match Tera::new("templates/**/*") {
            Ok(res) => res,
            Err(err) => panic!("Error on load tera templates: {}", err),
        };

        _tera.autoescape_on(vec![".jinja"]);
        _tera
    };
}
