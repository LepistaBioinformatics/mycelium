use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::Deserialize;
use std::path::PathBuf;
use toml::from_str;

/// Load configuration from TOML file
///
/// It is a generic function to read a configuration file from a TOML file.
pub fn load_config_from_file<T>(file_path: PathBuf) -> Result<T, MappedErrors>
where
    for<'a> T: Deserialize<'a>,
{
    let file_content =
        std::fs::read_to_string(file_path.as_path().to_str().unwrap())
            .expect("Could not read config file");

    let config: T = from_str(&file_content).map_err(|err| {
        dto_err(format!("Could not parse config file: {err}"))
    })?;

    Ok(config)
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::secret_resolver::SecretResolver;

    use super::*;
    use mycelium_base::utils::errors::MappedErrors;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct Config {
        name: String,
        age: u8,
        var_with_env: SecretResolver<String>,
        var_without_env: String,
    }

    #[tokio::test]
    async fn test_load_config_from_file() -> Result<(), MappedErrors> {
        std::env::set_var("ENV_VAR", "env_value");

        let config: Config =
            load_config_from_file(PathBuf::from("tests/config.yaml"))?;

        assert_eq!(config.name, "Name");
        assert_eq!(config.age, 99);
        assert_eq!(config.var_with_env.get_or_error()?, "env_value");
        assert_eq!(config.var_without_env, "value");

        Ok(())
    }
}
