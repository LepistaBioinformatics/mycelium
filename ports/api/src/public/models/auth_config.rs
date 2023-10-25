use crate::providers::{
    azure_config::AzureOauthConfig, google_config::GoogleOauthConfig,
};

use myc_config::optional_config::OptionalConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub internal: OptionalConfig<bool>,
    pub google: OptionalConfig<GoogleOauthConfig>,
    pub azure: OptionalConfig<AzureOauthConfig>,
}
