use crate::{optional_config::OptionalConfig, VaultConfig};

use lazy_static::lazy_static;
use std::{path::PathBuf, sync::Mutex};

lazy_static! {
    #[derive(Debug)]
    pub(crate) static ref VAULT_CONFIG: Mutex<Option<OptionalConfig<VaultConfig>>> = Mutex::new(None);
}

pub async fn init_vault_config_from_file(
    config_path: Option<PathBuf>,
    config_instance: Option<OptionalConfig<VaultConfig>>,
) {
    if let Some(config) = config_instance {
        VAULT_CONFIG.lock().unwrap().replace(config);
        return;
    }

    match config_path {
        None => {
            panic!(
                "Error detected on initialize smtp config: config path is none"
            );
        }
        Some(path) => VAULT_CONFIG.lock().unwrap().replace(
            match VaultConfig::from_default_config_file(path) {
                Ok(config) => config,
                Err(err) => {
                    panic!("Error detected on initialize smtp config: {err}")
                }
            },
        ),
    };
}

pub(crate) fn get_vault_config() -> OptionalConfig<VaultConfig> {
    VAULT_CONFIG
        .lock()
        .expect("Vault config not initialized")
        .as_ref()
        .expect("Vault config not initialized")
        .to_owned()
}
