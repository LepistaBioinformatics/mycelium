use serde::Serialize;
use std::fmt::{Display, Formatter, Result as FmtResult};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JsonError {
    msg: String,
    code: Option<String>,
}

impl JsonError {
    pub fn new(msg: String) -> Self {
        Self { msg, code: None }
    }

    pub fn with_code(&self, code: String) -> Self {
        Self {
            msg: self.msg.to_owned(),
            code: Some(code),
        }
    }
}

impl Display for JsonError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        if self.code.is_some() {
            return write!(f, "{}: {}", self.code.as_ref().unwrap(), self.msg);
        }

        write!(f, "{}", self.msg)
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_json_error() {
        let json_error = JsonError::new("test".to_string());
        assert_eq!(json_error.msg, "test");
        assert_eq!(json_error.code, None);
    }

    #[test]
    fn test_json_error_code() {
        let json_error =
            JsonError::new("test".to_string()).with_code("code".to_string());
        assert_eq!(json_error.msg, "test");
        assert_eq!(json_error.code, Some("code".to_string()));
    }

    #[test]
    fn test_json_error_display() {
        let json_error = JsonError::new("test".to_string());
        assert_eq!(format!("{}", json_error), "test");

        let json_error =
            JsonError::new("test".to_string()).with_code("code".to_string());
        assert_eq!(format!("{}", json_error), "code: test");
    }
}
