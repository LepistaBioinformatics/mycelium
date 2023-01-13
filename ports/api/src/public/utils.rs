use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct JsonError(pub String);
