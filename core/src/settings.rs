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

pub(crate) const DEFAULT_TENANT_ID_KEY: &str = "tenant_id";

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
            Err(err) => {
                tracing::warn!(
                    "Failed to load tera templates: {}. \
                     Template rendering will return errors until resolved.",
                    err
                );
                Tera::default()
            }
        };

        _tera.autoescape_on(vec![".jinja", ".subject"]);
        _tera
    };
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tera::Tera;

    #[test]
    fn test_tera_malformed_template_does_not_panic() {
        let tmp_dir =
            std::env::temp_dir().join("mycelium_test_tera_malformed");
        fs::create_dir_all(&tmp_dir).unwrap();
        let bad_tpl = tmp_dir.join("bad.html");
        fs::write(&bad_tpl, "{% this is not valid tera syntax %").unwrap();

        let result =
            Tera::new(&format!("{}/**/*", tmp_dir.to_string_lossy()));
        assert!(result.is_err(), "expected Tera to fail on malformed template");

        // Previously this arm called panic! — now it returns Tera::default()
        let tera = match result {
            Ok(t) => t,
            Err(_) => Tera::default(),
        };
        // Rendering on an empty Tera returns Err, not panic
        let render_result = tera.render("bad.html", &tera::Context::new());
        assert!(
            render_result.is_err(),
            "expected render to return Err when template is absent"
        );

        fs::remove_dir_all(&tmp_dir).ok();
    }
}
