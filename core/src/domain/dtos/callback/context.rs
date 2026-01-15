use crate::domain::dtos::{
    callback::UserInfo, http::HttpMethod, security_group::SecurityGroup,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallbackContext {
    pub status_code: u16,
    pub response_headers: HashMap<String, String>,
    pub duration_ms: u64,
    pub upstream_path: String,
    pub downstream_url: String,
    pub method: HttpMethod,
    pub timestamp: String,
    pub request_id: Option<String>,
    pub client_ip: Option<String>,
    pub user_info: Option<UserInfo>,
    pub security_group: SecurityGroup,
}

impl CallbackContext {
    pub fn new(
        status_code: u16,
        response_headers: HashMap<String, String>,
        duration_ms: u64,
        upstream_path: String,
        downstream_url: String,
        method: HttpMethod,
        timestamp: String,
        request_id: Option<String>,
        client_ip: Option<String>,
        user_info: Option<UserInfo>,
        security_group: SecurityGroup,
    ) -> Self {
        Self {
            status_code,
            response_headers,
            duration_ms,
            upstream_path,
            downstream_url,
            method,
            timestamp,
            request_id,
            client_ip,
            user_info,
            security_group,
        }
    }
}
