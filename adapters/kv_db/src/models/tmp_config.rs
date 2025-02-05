use super::QueueConfig;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TmpConfig {
    pub(super) queue: QueueConfig,
}
