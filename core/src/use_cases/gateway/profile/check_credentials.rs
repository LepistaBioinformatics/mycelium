use crate::{
    domain::dtos::email::Email, use_cases::gateway::token::decode_bearer_token,
};

use actix_web::{http::header::Header, HttpRequest};
use actix_web_httpauth::headers::authorization::{Authorization, Bearer};
use clean_base::utils::errors::{execution_err, MappedErrors};
use log::error;

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

    match decode_bearer_token(auth.as_ref().token().to_string()).await {
        Err(err) => return Err(err),
        Ok(res) => match Email::from_string(res.email) {
            Err(err) => {
                error!("{err}");
                return Err(execution_err(
                    "Unexpected error on check credentials from Bearer token."
                        .to_string(),
                    None,
                    None,
                ));
            }
            Ok(res) => Ok(res),
        },
    }
}
