use clean_base::utils::errors::MappedErrors;
use serde::Deserialize;

/// Load configuration from YAML file
///
/// It is a generic function to read a configuration file from a YAML file.
pub async fn load_config_from_file<T>(
    file_path: String,
) -> Result<T, MappedErrors>
where
    for<'a> T: Deserialize<'a>,
{
    let f = std::fs::File::open(file_path.as_str())
        .expect("Could not open config file.");

    let config: T =
        serde_yaml::from_reader(f).expect("Could not read config file.");

    Ok(config)
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use clean_base::utils::errors::MappedErrors;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct Config {
        name: String,
        age: u8,
    }

    #[tokio::test]
    async fn test_load_config_from_file() -> Result<(), MappedErrors> {
        let config: Config =
            load_config_from_file("tests/config.yaml".to_string())
                .await
                .unwrap();

        assert_eq!(config.name, "Name");
        assert_eq!(config.age, 99);

        Ok(())
    }
}
