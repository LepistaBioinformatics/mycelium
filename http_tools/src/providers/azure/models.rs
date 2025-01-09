use oauth2::{
    basic::BasicTokenType, ExtraTokenFields as _ExtraTokenFields,
    StandardTokenResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct MsGraphDecode {
    pub mail: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct QueryCode {
    pub code: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) struct ExtraTokenFields {}

impl _ExtraTokenFields for ExtraTokenFields {}

pub(super) type AzureTokenResponse =
    StandardTokenResponse<ExtraTokenFields, BasicTokenType>;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CsrfTokenClaims {
    pub(super) csrf: String,
    pub(super) exp: usize,
    pub(super) code_verifier: String,
}
