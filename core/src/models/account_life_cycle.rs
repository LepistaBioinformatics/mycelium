use myc_config::env_or_value::EnvOrValue;
use mycelium_base::utils::errors::MappedErrors;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// This struct is used to manage the token secret and the token expiration
/// times.
///
/// This is not the final position of this struct, it will be moved to a
/// dedicated module in the future.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountLifeCycle {
    /// Token expiration time in seconds
    ///
    /// This information is used to calculate the lifetime for new user
    /// registration
    pub token_expiration: i64,

    /// General Purpose email name
    pub noreply_name: Option<String>,

    /// General Purpose email
    pub noreply_email: EnvOrValue<String>,

    /// Support email name
    pub support_name: Option<String>,

    /// Support email
    pub support_email: EnvOrValue<String>,

    /// Token secret
    ///
    /// Toke secret is used to sign tokens
    pub(crate) token_secret: EnvOrValue<Uuid>,
}

impl AccountLifeCycle {
    pub fn get_secret(&self) -> Result<Uuid, MappedErrors> {
        self.token_secret.get_or_error()
    }
}
