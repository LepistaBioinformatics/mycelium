use myc_config::env_or_value::EnvOrValue;
use serde::{Deserialize, Serialize};

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

    /// General Purpose email
    pub noreply_email: EnvOrValue<String>,

    /// Support email
    pub support_email: EnvOrValue<String>,
}
