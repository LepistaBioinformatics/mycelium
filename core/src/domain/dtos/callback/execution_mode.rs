use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionMode {
    Parallel,
    Sequential,

    #[default]
    FireAndForget,
}
