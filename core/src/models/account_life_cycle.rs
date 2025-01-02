use myc_config::secret_resolver::SecretResolver;
use serde::{Deserialize, Serialize};

/// This struct is used to manage the token secret and the token expiration
/// times.
///
/// This is not the final position of this struct, it will be moved to a
/// dedicated module in the future.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountLifeCycle {
    /// Domain name
    pub domain_name: SecretResolver<String>,

    /// Domain URL
    pub domain_url: Option<SecretResolver<String>>,

    /// Default language
    pub locale: Option<SecretResolver<String>>,

    /// Token expiration time in seconds
    ///
    /// This information is used to calculate the lifetime for new user
    /// registration
    pub token_expiration: SecretResolver<i64>,

    /// General Purpose email name
    pub noreply_name: Option<SecretResolver<String>>,

    /// General Purpose email
    pub noreply_email: SecretResolver<String>,

    /// Support email name
    pub support_name: Option<SecretResolver<String>>,

    /// Support email
    pub support_email: SecretResolver<String>,

    /// Token secret
    ///
    /// Toke secret is used to sign tokens
    pub(crate) token_secret: SecretResolver<String>,
}
