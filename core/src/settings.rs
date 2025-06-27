use lazy_static::lazy_static;
use std::env::{self};
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
