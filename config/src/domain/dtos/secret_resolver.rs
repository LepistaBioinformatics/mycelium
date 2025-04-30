use crate::{get_vault_config, optional_config::OptionalConfig};

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, str::FromStr};
use utoipa::ToSchema;

/// A secret resolver
///
/// The secret resolver is a way to resolve a secret value from different
/// sources.
///
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
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
    ///     vault:
    ///         path: "my_vault_secret"
    ///         key: "my_key"
    /// ```
    ///
    /// The value of `databaseUrl` will be the value of the secret located at
    /// `path/my_vault_secret` in the vault.
    ///
    Vault { path: String, key: String },

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
            //
            // Return the value directly
            //
            SecretResolver::Value(value) => Ok(value.to_owned()),
            //
            // Retrieve the value from the environment variable
            //
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
            //
            // Retrieve value from the vault using simple HTTP GET
            //
            SecretResolver::Vault { path, key } => {
                panic!(
                    "Vault config should not be used in sync context: {path}/{key}",
                    path = path,
                    key = key
                )
            }
        }
    }

    #[tracing::instrument(name = "async_get_or_error", skip_all)]
    pub async fn async_get_or_error(&self) -> Result<T, MappedErrors> {
        match self {
            //
            // Return the value directly
            //
            SecretResolver::Value(value) => Ok(value.to_owned()),
            //
            // Retrieve the value from the environment variable
            //
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
            //
            // Retrieve value from the vault using simple HTTP GET
            //
            SecretResolver::Vault { path, key } => {
                tracing::trace!(
                    "Resolving remote secret from vault: {path}/{key}",
                    path = path,
                    key = key
                );
                //
                // Get the vault configuration
                //
                let config = match get_vault_config() {
                    OptionalConfig::Disabled => {
                        panic!("Vault config not initialized")
                    }
                    OptionalConfig::Enabled(config) => config,
                };

                let token = match config.token {
                    SecretResolver::Vault { path, key } => {
                        return execution_err(format!(
                            "Vault config should not be used to initialize vault client: trying {path}/{key}",
                            path = path,
                            key = key
                        ))
                            .as_error()
                    },
                    _ => config.token.get_or_error()?
                };

                //
                // Fetch the secret from the vault
                //
                let response = match Client::new()
                    .get(format!(
                        "{}/{}/data/{}",
                        config.url,
                        config.version_with_namespace,
                        path.to_owned()
                    ))
                    .header("X-Vault-Token", token)
                    .send()
                    .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        return execution_err(format!(
                            "Could not fetch secret from vault: {err}"
                        ))
                        .as_error()
                    }
                };

                //
                // Check the response status
                //
                match response.status() {
                    reqwest::StatusCode::OK => {}
                    _ => {
                        return execution_err(format!(
                            "Invalid vault response. Please verify the vault connection credentials: status {status}",
                            status = response.status()
                        ))
                        .as_error()
                    }
                }

                //
                // Extract response JSON
                //
                let vault_response =
                    match response.json::<VaultResponse>().await {
                        Ok(res) => res,
                        Err(err) => {
                            return execution_err(format!(
                                "Unable to parse vault response: {err}"
                            ))
                            .as_error()
                        }
                    };

                let binding = key.clone();
                let search_key = binding.as_str();

                let _value = match vault_response.data.data.get(search_key) {
                    Some(value) => value.to_owned(),
                    None => {
                        return execution_err(
                            format!("Invalid vault secret path. Please verify: {path}/{key}",
                                path = path,
                                key = key
                            )
                        )
                        .as_error()
                    }
                };

                //
                // Parse the response
                //
                match _value.parse::<T>() {
                    Ok(res) => Ok(res),
                    Err(_) => execution_err(format!(
                        "Could not parse vault secret: {path}/{key}",
                        path = path,
                        key = key
                    ))
                    .as_error(),
                }
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct VaultResponse {
    data: VaultData,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
struct VaultData {
    data: HashMap<String, String>,
}
