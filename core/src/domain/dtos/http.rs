//use http::Method as HttpCrateMethod;
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
    Read,
    Write,
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

    pub fn is_read_method(&self) -> bool {
        matches!(
            self,
            HttpMethod::Get
                | HttpMethod::Head
                | HttpMethod::Options
                | HttpMethod::Trace
        )
    }

    pub fn is_write_method(&self) -> bool {
        matches!(
            self,
            HttpMethod::Post
                | HttpMethod::Put
                | HttpMethod::Patch
                | HttpMethod::Delete
        )
    }

    pub fn get_read_methods(&self) -> Vec<HttpMethod> {
        vec![
            HttpMethod::Get,
            HttpMethod::Head,
            HttpMethod::Options,
            HttpMethod::Trace,
        ]
    }

    pub fn get_write_methods(&self) -> Vec<HttpMethod> {
        vec![
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
        ]
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
            HttpMethod::Read => write!(f, "READ"),
            HttpMethod::Write => write!(f, "WRITE"),
        }
    }
}

impl FromStr for HttpMethod {
    type Err = HttpMethod;

    fn from_str(s: &str) -> Result<HttpMethod, HttpMethod> {
        match s {
            "GET" | "get" => Ok(HttpMethod::Get),
            "HEAD" | "head" => Ok(HttpMethod::Head),
            "PATCH" | "patch" => Ok(HttpMethod::Patch),
            "POST" | "post" => Ok(HttpMethod::Post),
            "PUT" | "put" => Ok(HttpMethod::Put),
            "DELETE" | "delete" => Ok(HttpMethod::Delete),
            "CONNECT" | "connect" => Ok(HttpMethod::Connect),
            "OPTIONS" | "options" => Ok(HttpMethod::Options),
            "TRACE" | "trace" => Ok(HttpMethod::Trace),
            "ALL" | "all" => Ok(HttpMethod::All),
            "READ" | "read" => Ok(HttpMethod::Read),
            "WRITE" | "write" => Ok(HttpMethod::Write),
            "NONE" | "none" => Ok(HttpMethod::None),
            _ => Err(HttpMethod::None),
        }
    }
}

impl From<HttpMethod> for Method {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => Method::GET,
            HttpMethod::Head => Method::HEAD,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Delete => Method::DELETE,
            HttpMethod::Connect => Method::CONNECT,
            HttpMethod::Options => Method::OPTIONS,
            HttpMethod::Trace => Method::TRACE,
            _ => Method::GET,
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
