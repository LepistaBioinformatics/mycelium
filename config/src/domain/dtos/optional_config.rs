use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OptionalConfig<T> {
    Disabled,

    #[serde(untagged)]
    Enabled(T),
}
