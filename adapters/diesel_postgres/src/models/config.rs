use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use myc_config::{load_config_from_file, secret_resolver::SecretResolver};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde::Deserialize;
use shaku::Interface;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DieselConfig {
    pub database_url: SecretResolver<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    diesel: DieselConfig,
}

impl DieselConfig {
    pub fn from_default_config_file(
        file: PathBuf,
    ) -> Result<Self, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        match load_config_from_file::<TmpConfig>(file) {
            Ok(config) => Ok(config.diesel),
            Err(err) => Err(err),
        }
    }
}

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

pub trait DbPoolProvider: Interface + Send + Sync {
    fn get_pool(&self) -> DbPool;
}
