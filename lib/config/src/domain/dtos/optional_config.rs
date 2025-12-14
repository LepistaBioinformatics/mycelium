use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OptionalConfig<T> {
    Disabled,

    #[serde(alias = "define", alias = "set")]
    Enabled(T),
}

impl<T> Default for OptionalConfig<T> {
    fn default() -> Self {
        Self::Disabled
    }
}
