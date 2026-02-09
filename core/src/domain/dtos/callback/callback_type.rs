use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CallbackType {
    Rhai,
    Javascript,
    Python,
    Http,
}
