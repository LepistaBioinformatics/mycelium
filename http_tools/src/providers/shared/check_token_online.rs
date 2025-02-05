use crate::settings::get_client;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::{IntoUrl, StatusCode};
use serde::Deserialize;

/// Check token online
///
/// This function is used to check the token online. The real implementation
/// should try to collect the user credentials from the request and return the
/// user email as response.
pub(crate) async fn check_token_online<T, U>(
    token: String,
    url: U,
    as_query_param: Option<bool>,
) -> Result<T, MappedErrors>
where
    T: for<'a> Deserialize<'a> + 'static,
    U: IntoUrl,
{
    let client = get_client().await;
    let response = match (if let Some(true) = as_query_param {
        client.get(url).query(&[("id_token", token)])
    } else {
        client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
    })
    .send()
    .await
    {
        Err(err) => {
            return execution_err(format!("Invalid client request: {err}"))
                .as_error()
        }
        Ok(res) => res,
    };

    let status = response.status();

    match status {
        // 2xx status code
        StatusCode::NOT_FOUND => {
            return execution_err(format!("Invalid user.")).as_error()
        }
        StatusCode::OK => match response.json::<T>().await {
            Err(err) => {
                return execution_err(format!(
                    "Unexpected error on fetch user info online: {err}"
                ))
                .as_error()
            }
            Ok(res) => Ok(res),
        },
        // 4xx status code
        StatusCode::UNAUTHORIZED => {
            let msg = response.text().await.unwrap_or("No message".to_string());

            return execution_err(format!(
                "Unauthorized user ({:?}): {:?}",
                status, msg
            ))
            .as_error();
        }
        StatusCode::FORBIDDEN => {
            let msg = response.text().await.unwrap_or("No message".to_string());

            return execution_err(format!(
                "Forbidden user ({:?}): {:?}",
                status, msg
            ))
            .as_error();
        }
        // Other
        _ => {
            tracing::warn!(
                "Unexpected error on fetch user info online (status {:?}): {:?}",
                status,
                response.text().await
            );

            return execution_err(
                "Unexpected error on fetch user info online.".to_string(),
            )
            .as_error();
        }
    }
}
