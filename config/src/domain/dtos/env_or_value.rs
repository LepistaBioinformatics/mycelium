use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

/// A value that can be either an environment variable or a value.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EnvOrValue<T> {
    Env(String),

    #[serde(untagged)]
    Value(T),
}

impl<T: FromStr + Debug + Clone> EnvOrValue<T> {
    /// Returns the value of the environment variable if it exists, otherwise
    /// returns the value.
    pub fn get_or_error(&self) -> Result<T, MappedErrors> {
        match self {
            EnvOrValue::Env(env) => match std::env::var(env) {
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
            EnvOrValue::Value(value) => Ok(value.to_owned()),
        }
    }
}
