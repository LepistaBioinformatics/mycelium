use oauth2::{
    basic::BasicTokenType, ExtraTokenFields as _ExtraTokenFields,
    StandardTokenResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct MsGraphDecode {
    pub mail: String,
}

//#[derive(Debug, Serialize, Deserialize)]
//pub(super) struct AzureTokenResponse {
//    pub(super) access_token: String,
//    pub(super) id_token: String,
//    pub(super) expires_in: i64,
//    pub(super) scope: String,
//    pub(super) token_type: String,
//}

#[derive(Debug, Deserialize)]
pub(super) struct QueryCode {
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) struct ExtraTokenFields {}

impl _ExtraTokenFields for ExtraTokenFields {}

pub(super) type AzureTokenResponse =
    StandardTokenResponse<ExtraTokenFields, BasicTokenType>;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CsrfTokenClaims {
    pub(super) csrf: String, // Um identificador exclusivo
    pub(super) exp: usize,   // Data de expiração
}
