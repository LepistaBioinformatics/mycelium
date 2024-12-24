use myc_config::secret_resolver::SecretResolver;
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
    /// Domain name
    pub domain_name: String,

    /// Domain URL
    pub domain_url: Option<String>,

    /// Default language
    pub locale: Option<String>,

    /// Token expiration time in seconds
    ///
    /// This information is used to calculate the lifetime for new user
    /// registration
    pub token_expiration: i64,

    /// General Purpose email name
    pub noreply_name: Option<String>,

    /// General Purpose email
    pub noreply_email: SecretResolver<String>,

    /// Support email name
    pub support_name: Option<String>,

    /// Support email
    pub support_email: SecretResolver<String>,

    /// Token secret
    ///
    /// Toke secret is used to sign tokens
    pub(crate) token_secret: SecretResolver<Uuid>,
}

impl AccountLifeCycle {
    pub fn get_secret(&self) -> Result<Uuid, MappedErrors> {
        self.token_secret.get_or_error()
    }
}
