use crate::settings::get_client;

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use clean_base::utils::errors::{execution_err, MappedErrors};
use log::warn;
use myc_core::domain::dtos::email::Email;
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct GcGraphDecode {
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
            return Err(execution_err(
                format!("Invalid client request: {err}"),
                Some(true),
                None,
            ));
        }
        Ok(res) => res,
    };

    decode_bearer_token_on_google(auth.to_owned()).await
}

/// Decode the bearer token on Google.
///
/// This function is used to decode the bearer token on Google.
/// The real implementation should try to collect the user credentials from the
/// request and return the user email as response.
///
async fn decode_bearer_token_on_google(
    auth: Authorization<Bearer>,
) -> Result<Email, MappedErrors> {
    let response = match get_client()
        .await
        .get("https://people.googleapis.com/v1/people/me")
        .header("Authorization", auth.to_string())
        .send()
        .await
    {
        Err(err) => {
            return Err(execution_err(
                format!("Invalid client request: {err}"),
                Some(true),
                None,
            ))
        }
        Ok(res) => res,
    };

    match response.status() {
        StatusCode::NOT_FOUND => {
            return Err(execution_err(
                format!("Invalid user."),
                Some(true),
                None,
            ))
        }
        StatusCode::OK => {
            let res = match response.json::<GcGraphDecode>().await {
                Err(err) => {
                    return Err(execution_err(
                        format!(
                            "Unexpected error on fetch user from MS Graph: {err}"
                        ),
                        Some(true),
                        None,
                    ))
                }
                Ok(res) => match Email::from_string(res.mail) {
                    Err(err) => {
                        return Err(execution_err(
                            format!("Unexpected error on parse user from MS Graph: {err}"),
                            Some(true),
                            None,
                        ))
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

            return Err(execution_err(
                "Unexpected error on fetch user from MS Graph.".to_string(),
                Some(true),
                None,
            ));
        }
    }
}
