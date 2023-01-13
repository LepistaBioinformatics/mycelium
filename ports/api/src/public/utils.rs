use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JsonError {
    msg: String,
}

impl JsonError {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}
