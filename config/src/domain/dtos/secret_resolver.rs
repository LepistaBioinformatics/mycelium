use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};
use utoipa::ToSchema;

/// A secret resolver
///
/// The secret resolver is a way to resolve a secret value from different
/// sources.
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum SecretResolver<T> {
    /// Retrieve the value from the environment variable
    ///
    /// The value should be the name of the environment variable.
    ///
    /// # Example
    ///
    /// ```bash
    /// export MY_ENV_VAR="my_value"
    /// ```
    ///
    /// ```yaml
    /// databaseUrl:
    ///     env: "MY_ENV_VAR"
    /// ```
    ///
    /// The value of `databaseUrl` will be `my_value`
    ///
    Env(String),

    /// Retrieve the value from the vault
    ///
    /// The value should be the name of the vault secret.
    ///
    /// # Example
    ///
    /// ```yaml
    /// databaseUrl:
    ///     vault: "path/my_vault_secret"
    /// ```
    ///
    /// The value of `databaseUrl` will be the value of the secret located at
    /// `path/my_vault_secret` in the vault.
    ///
    Vault(String),

    /// Retrieve the value directly from a configuration file
    ///
    /// The value should be the value itself.
    ///
    /// # Example
    ///
    /// ```yaml
    /// databaseUrl: "my_value"
    /// ```
    ///
    #[serde(untagged)]
    Value(T),
}

impl<T: FromStr + Debug + Clone> SecretResolver<T> {
    /// Returns the value of the environment variable if it exists, otherwise
    /// returns the value.
    pub fn get_or_error(&self) -> Result<T, MappedErrors> {
        match self {
            SecretResolver::Value(value) => Ok(value.to_owned()),
            SecretResolver::Env(env) => match std::env::var(env) {
                Ok(value) => match value.parse::<T>() {
                    Ok(res) => Ok(res),
                    Err(_) => execution_err(format!(
                        "Could not parse environment variable {env}: {value}"
                    ))
                    .as_error(),
                },
                Err(err) => execution_err(format!(
                    "Could not parse environment variable {env}: {err}"
                ))
                .as_error(),
            },
            SecretResolver::Vault(_) => {
                unimplemented!("Vault is not implemented yet");
            }
        }
    }
}
