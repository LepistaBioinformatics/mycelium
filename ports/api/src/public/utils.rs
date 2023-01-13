use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct JsonError {
    msg: String,
}

impl JsonError {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}
