use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CallbackStatement {
    OneOf,
    AllOf,
    NoneOf,
}
