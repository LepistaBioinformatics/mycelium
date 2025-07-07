use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Copy, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Location {
    Query,
    Path,
    Header,
    Cookie,
}
