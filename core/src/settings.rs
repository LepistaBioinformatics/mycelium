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

pub const DEFAULT_PROFILE_KEY: &str = "x-mycelium-profile";

pub const FORWARD_FOR_KEY: &str = "x-forwarded-for";

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
