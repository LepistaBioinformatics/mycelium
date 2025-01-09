mod json_response;

pub use json_response::HttpJsonResponse;

#[deprecated(note = "Use HttpJsonResponse instead")]
pub use HttpJsonResponse as JsonError;
