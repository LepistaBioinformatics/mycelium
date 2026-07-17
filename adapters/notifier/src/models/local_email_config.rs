#![cfg(feature = "local-transport")]

use myc_config::{load_config_from_file, optional_config::OptionalConfig};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

/// Opt-in local file delivery for standalone builds: when set, undelivered
/// mail is written as one `.eml` file per message under `dir` instead of being
/// rendered to the terminal by the stub transport. Selection precedence stays
/// SMTP > File > Stub (see `select_local_transport`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEmailConfig {
    pub dir: PathBuf,
}

// Loaded on its own so an absent `[localEmail]` table resolves to `Disabled`
// rather than a load error -- file delivery is opt-in, same as standalone's
// `[smtp]`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OptionalTmpConfig {
    #[serde(default)]
    local_email: OptionalConfig<LocalEmailConfig>,
}

impl LocalEmailConfig {
    /// Load `[localEmail]` as optional -- absent section resolves to
    /// `Disabled`, letting `select_local_transport` fall through to the stub.
    pub fn from_optional_config_file(
        file: PathBuf,
    ) -> Result<OptionalConfig<Self>, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        Ok(load_config_from_file::<OptionalTmpConfig>(file)?.local_email)
    }

    /// Resolve the delivery directory, creating it (and any missing parents)
    /// if absent. lettre's `FileTransport` does not create the directory --
    /// it errors on the first send if it is missing -- so we create it
    /// up-front, matching standalone's "created on first boot" convention for
    /// the sqlite file.
    pub fn ensure_dir(&self) -> Result<PathBuf, MappedErrors> {
        fs::create_dir_all(&self.dir).map_err(|err| {
            creation_err(format!(
                "Could not create local email dir {}: {err}",
                self.dir.display()
            ))
        })?;

        Ok(self.dir.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn ensure_dir_creates_missing_nested_directory() {
        let base = std::env::temp_dir()
            .join(format!("myc_local_email_{}", Uuid::new_v4()));
        let nested = base.join("emails");
        assert!(!nested.exists());

        let config = LocalEmailConfig {
            dir: nested.clone(),
        };
        let resolved = config.ensure_dir().unwrap();

        assert_eq!(resolved, nested);
        assert!(nested.is_dir());

        let _ = fs::remove_dir_all(&base);
    }

    // Guards the exact `[localEmail.define]` shape documented in the example
    // config against future serde-tag surprises (`OptionalConfig`'s `Enabled`
    // is aliased to `define`/`set`, not a bare table).
    #[test]
    fn define_table_shape_resolves_to_enabled() {
        use std::io::Write;

        let file = std::env::temp_dir()
            .join(format!("myc_local_email_cfg_{}.toml", Uuid::new_v4()));
        let mut handle = fs::File::create(&file).unwrap();
        handle
            .write_all(b"[localEmail.define]\ndir = \"/tmp/myc-emails\"\n")
            .unwrap();

        let resolved =
            LocalEmailConfig::from_optional_config_file(file.clone()).unwrap();

        let OptionalConfig::Enabled(config) = resolved else {
            panic!("[localEmail.define] should resolve to Enabled");
        };
        assert_eq!(config.dir, PathBuf::from("/tmp/myc-emails"));

        let _ = fs::remove_file(&file);
    }
}
