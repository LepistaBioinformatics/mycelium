use actix_web::HttpResponse;
use serde::Serialize;
use std::fmt::{Display, Formatter, Result as FmtResult};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpJsonResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    msg: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

impl HttpJsonResponse {
    pub fn new_message<T: ToString>(msg: T) -> Self {
        Self {
            msg: msg.to_string().into(),
            code: None,
            body: None,
        }
    }

    pub fn new_body<T: Serialize>(body: T) -> Result<Self, HttpResponse> {
        match serde_json::to_string(&body) {
            Err(err) => {
                let json_error = HttpJsonResponse::new_message(err.to_string());

                return Err(
                    HttpResponse::InternalServerError().json(json_error)
                );
            }
            Ok(body) => Ok(Self {
                msg: None,
                code: None,
                body: Some(body),
            }),
        }
    }

    pub fn new_vec_body<T: Serialize>(
        body: Vec<T>,
    ) -> Result<Self, HttpResponse> {
        match serde_json::to_string(&body) {
            Err(err) => {
                let json_error = HttpJsonResponse::new_message(err.to_string());

                return Err(
                    HttpResponse::InternalServerError().json(json_error)
                );
            }
            Ok(body) => Ok(Self {
                msg: None,
                code: None,
                body: Some(body),
            }),
        }
    }

    pub fn with_code<T: ToString>(&self, code: T) -> Self {
        Self {
            msg: self.msg.to_owned(),
            code: Some(code.to_string()),
            body: self.body.to_owned(),
        }
    }

    pub fn with_code_str(&self, code: &'static str) -> Self {
        Self {
            msg: self.msg.to_owned(),
            code: Some(code.to_string()),
            body: self.body.to_owned(),
        }
    }

    pub fn with_body(&self, body: String) -> Self {
        Self {
            msg: self.msg.to_owned(),
            code: self.code.to_owned(),
            body: Some(body),
        }
    }

    pub fn with_serializable_body<T: Serialize>(
        &self,
        body: T,
    ) -> Result<Self, HttpResponse> {
        match serde_json::to_string(&body) {
            Err(err) => {
                let json_error = HttpJsonResponse::new_message(err.to_string());

                return Err(
                    HttpResponse::InternalServerError().json(json_error)
                );
            }
            Ok(body) => Ok(Self {
                msg: self.msg.to_owned(),
                code: self.code.to_owned(),
                body: Some(body),
            }),
        }
    }
}

impl Display for HttpJsonResponse {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let msg = self.msg.to_owned().unwrap_or("no message".to_string());

        if self.code.is_some() {
            return write!(f, "{}: {}", self.code.as_ref().unwrap(), msg);
        }

        write!(f, "{}", msg)
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
        let json_error = HttpJsonResponse::new_message("test".to_string());
        assert_eq!(json_error.msg, Some("test".to_string()));
        assert_eq!(json_error.code, None);
    }

    #[test]
    fn test_json_error_code() {
        let json_error = HttpJsonResponse::new_message("test".to_string())
            .with_code_str("code");
        assert_eq!(json_error.msg, Some("test".to_string()));
        assert_eq!(json_error.code, Some("code".to_string()));
    }

    #[test]
    fn test_json_error_display() {
        let json_error = HttpJsonResponse::new_message("test".to_string());
        assert_eq!(format!("{}", json_error), "test");

        let json_error = HttpJsonResponse::new_message("test".to_string())
            .with_code_str("code");
        assert_eq!(format!("{}", json_error), "code: test");
    }
}
