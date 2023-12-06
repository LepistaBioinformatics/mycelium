use clean_base::utils::errors::{factories::creation_err, MappedErrors};
use myc_config::{env_or_value::EnvOrValue, load_config_from_file};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismaConfig {
    pub database_url: EnvOrValue<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    prisma: PrismaConfig,
}

impl PrismaConfig {
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
            Ok(config) => Ok(config.prisma),
            Err(err) => Err(err),
        }
    }
}
