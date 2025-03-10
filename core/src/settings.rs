use crate::{
    domain::dtos::route::Route,
    use_cases::gateway::routes::load_config_from_yaml,
};

use futures::lock::Mutex;
use lazy_static::lazy_static;
use std::env::{self, var_os};
use tera::Tera;

// ? ---------------------------------------------------------------------------
// ? Configure default system constants
// ? ---------------------------------------------------------------------------

/// Default TOTP domain
///
/// This is the default domain used to generate the TOTP token.
///
pub const DEFAULT_TOTP_DOMAIN: &str = "Mycelium";

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
        let template_dir = env::var("TEMPLATES_DIR")
            .unwrap_or_else(|_| "templates".to_string());

        tracing::info!("Loading templates from: {}", template_dir);

        let mut _tera = match Tera::new(&format!("{}/{}", template_dir, "**/*"))
        {
            Ok(res) => res,
            Err(err) => panic!("Error on load tera templates: {}", err),
        };

        _tera.autoescape_on(vec![".jinja", ".subject"]);
        _tera
    };
}
