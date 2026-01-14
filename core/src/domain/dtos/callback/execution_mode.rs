use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionMode {
    Parallel,
    Sequential,
    FireAndForget,
}
