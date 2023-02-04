use super::{health_check::HealthCheckConfig, http::Protocol, route::Route};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileService {
    pub id: Option<Uuid>,
    pub host: String,
    pub profile_validation_path: String,
    pub token_validation_path: String,
    pub attempts: i32,
    pub protocol: Protocol,
    pub health_check: HealthCheckConfig,
}

impl ProfileService {
    pub fn build_profiles_url(&self) -> String {
        format!(
            "{}://{}{}",
            self.protocol, self.host, self.profile_validation_path
        )
    }

    pub fn build_tokens_url(&self) -> String {
        format!(
            "{}://{}{}",
            self.protocol, self.host, self.token_validation_path
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientService {
    pub id: Option<Uuid>,
    pub name: String,
    pub host: String,
    pub health_check: Option<HealthCheckConfig>,
    pub routes: Vec<Route>,
}
