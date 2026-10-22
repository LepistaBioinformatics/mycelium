use crate::domain::dtos::hosted_login::HostedLoginClients;

use serde::{Deserialize, Serialize};

/// `[auth.hostedLogin.define]` — clients allowed to use the gateway-hosted
/// email login pages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedLoginConfig {
    #[serde(default)]
    pub clients: HostedLoginClients,
}
