use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use derive_more::Display;
use serde::Serialize;
use std::fmt::Debug;

/// The default JSON response from API errors
///
/// Use it in replacement of the simple html body.
#[derive(Debug, Serialize)]
pub(crate) struct JsonError {
    msg: String,
    status: u16,
    message: String,
}

/// Internal errors as HTTP responses
///
/// Forwarding errors are fired only by Stomata errors.
#[derive(Debug, Display)]
pub enum GatewayError {
    // ? -----------------------------------------------------------------------
    // ? Client errors (4xx)
    // ? -----------------------------------------------------------------------
    #[display(fmt = "BadRequest")]
    BadRequest(String),

    #[display(fmt = "Forbidden")]
    Forbidden(String),

    #[display(fmt = "Unauthorized")]
    Unauthorized(String),

    // ? -----------------------------------------------------------------------
    // ? Server errors (5xx)
    // ? -----------------------------------------------------------------------
    #[display(fmt = "InternalServerError")]
    InternalServerError(String),
}

impl error::ResponseError for GatewayError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(JsonError {
                msg: self.to_string(),
                status: self.status_code().as_u16(),
                message: match self {
                    GatewayError::BadRequest(msg) => msg.to_owned(),
                    GatewayError::Forbidden(msg) => msg.to_owned(),
                    GatewayError::Unauthorized(msg) => msg.to_owned(),
                    GatewayError::InternalServerError(msg) => msg.to_owned(),
                },
            })
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            GatewayError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            GatewayError::Forbidden { .. } => StatusCode::FORBIDDEN,
            GatewayError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            GatewayError::InternalServerError { .. } => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
