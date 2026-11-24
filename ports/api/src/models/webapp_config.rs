use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `[api.webapp]`: the webapp build the gateway serves under `/_app/`
///
/// Needs the `webapp` feature; the boot is refused when this section is set
/// without it (WEBAPP-09).
///
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebappConfig {
    /// The webapp `dist/`, built with `VITE_BASE_PATH=/_app/`
    pub dist_dir: PathBuf,

    /// The hosted login client the webapp signs in with, published by
    /// `GET /app-config` as `hostedLoginClientId`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_login_client_id: Option<String>,
}
