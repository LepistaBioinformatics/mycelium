use crate::providers::shared::check_token_online;

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use myc_core::domain::dtos::email::Email;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct MsGraphDecode {
    pub mail: String,
}

/// Try to collect the user email.
///
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
pub async fn check_credentials(
    req: HttpRequest,
) -> Result<Email, MappedErrors> {
    let auth = match Authorization::<Bearer>::parse(&req) {
        Err(err) => {
            return execution_err(format!("Invalid client request: {err}"))
                .as_error();
        }
        Ok(res) => res,
    };

    decode_bearer_token_on_ms_graph(auth.to_owned()).await
}

/// Decode the bearer token on MS Graph.
///
/// This function is used to decode the bearer token on MS Graph.
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
///
async fn decode_bearer_token_on_ms_graph(
    auth: Authorization<Bearer>,
) -> Result<Email, MappedErrors> {
    let token = auth.into_scheme().token().to_string();
    let token_response: MsGraphDecode =
        check_token_online(token, "https://graph.microsoft.com/v1.0/me/", None)
            .await?;

    Email::from_string(token_response.mail)
}
