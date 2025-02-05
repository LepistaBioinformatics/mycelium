use super::{QueueConfig, SmtpConfig};

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TmpConfig {
    pub(super) smtp: SmtpConfig,
    pub(super) queue: QueueConfig,
}
