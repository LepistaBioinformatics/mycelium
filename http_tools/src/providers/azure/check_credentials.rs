use crate::settings::get_client;

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use log::warn;
use myc_core::domain::dtos::email::Email;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::StatusCode;
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
    let response = match get_client()
        .await
        .get("https://graph.microsoft.com/v1.0/me/")
        .header("Authorization", auth.to_string())
        .send()
        .await
    {
        Err(err) => {
            return execution_err(format!("Invalid client request: {err}"))
                .as_error()
        }
        Ok(res) => res,
    };

    match response.status() {
        StatusCode::NOT_FOUND => {
            return execution_err(format!("Invalid user.")).as_error()
        }
        StatusCode::OK => {
            let res = match response.json::<MsGraphDecode>().await {
                Err(err) => {
                    return execution_err(format!(
                        "Unexpected error on fetch user from MS Graph: {err}"
                    ))
                    .as_error()
                }
                Ok(res) => match Email::from_string(res.mail) {
                    Err(err) => {
                        return execution_err(format!(
                        "Unexpected error on parse user from MS Graph: {err}"
                    ))
                        .as_error()
                    }
                    Ok(res) => res,
                },
            };

            return Ok(res);
        }
        _ => {
            warn!(
                "Unexpected error on fetch user from MS Graph (status {:?}) {:?}",
                response.status(),
                response.text().await
            );

            return execution_err(
                "Unexpected error on fetch user from MS Graph.".to_string(),
            )
            .as_error();
        }
    }
}
