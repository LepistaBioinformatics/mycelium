use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Head,
    Patch,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Protocol {
    Http,
    Https,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Protocol::Http => write!(f, "http"),
            Protocol::Https => write!(f, "https"),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RouteType {
    Public,
    Protected,
}
