use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::{ToResponse, ToSchema};

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ToSchema,
    ToResponse,
)]
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
    All,
    None,
}

impl HttpMethod {
    /// Convert a reqwest::Method to a HttpMethod.
    pub fn from_reqwest_method(method: Method) -> HttpMethod {
        match method {
            Method::GET => HttpMethod::Get,
            Method::HEAD => HttpMethod::Head,
            Method::PATCH => HttpMethod::Patch,
            Method::POST => HttpMethod::Post,
            Method::PUT => HttpMethod::Put,
            Method::DELETE => HttpMethod::Delete,
            Method::CONNECT => HttpMethod::Connect,
            Method::OPTIONS => HttpMethod::Options,
            Method::TRACE => HttpMethod::Trace,
            _ => HttpMethod::None,
        }
    }
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Connect => write!(f, "CONNECT"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Trace => write!(f, "TRACE"),
            HttpMethod::All => write!(f, "ALL"),
            HttpMethod::None => write!(f, "NONE"),
        }
    }
}

impl FromStr for HttpMethod {
    type Err = HttpMethod;

    fn from_str(s: &str) -> Result<HttpMethod, HttpMethod> {
        match s {
            "GET" => Ok(HttpMethod::Get),
            "HEAD" => Ok(HttpMethod::Head),
            "PATCH" => Ok(HttpMethod::Patch),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "CONNECT" => Ok(HttpMethod::Connect),
            "OPTIONS" => Ok(HttpMethod::Options),
            "TRACE" => Ok(HttpMethod::Trace),
            "ALL" => Ok(HttpMethod::All),
            "NONE" => Ok(HttpMethod::None),
            _ => Err(HttpMethod::None),
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    ToSchema,
    ToResponse,
    PartialEq,
    Eq,
)]
#[serde(rename_all = "camelCase")]
pub enum Protocol {
    Http,
    Https,
    Grpc,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Protocol::Http => write!(f, "http"),
            Protocol::Https => write!(f, "https"),
            Protocol::Grpc => write!(f, "grpc"),
        }
    }
}
