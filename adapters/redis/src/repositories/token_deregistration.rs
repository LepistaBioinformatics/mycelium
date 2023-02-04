use super::functions::{from_redis_key, get_today_key};
use crate::repositories::connector::get_connection;

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use clean_base::{
    entities::default_response::FetchResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use log::{debug, warn};
use myc_core::domain::{dtos::token::Token, entities::TokenDeregistration};
use redis::ErrorKind;
use shaku::Component;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenDeregistration)]
pub struct TokenDeregistrationMemDbRepository {}

#[async_trait]
impl TokenDeregistration for TokenDeregistrationMemDbRepository {
    async fn get_then_delete(
        &self,
        token: Token,
    ) -> Result<FetchResponseKind<Token, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build connection
        // ? -------------------------------------------------------------------

        let mut conn = get_connection().await;

        // ? -------------------------------------------------------------------
        // ? Try to persist data
        // ? -------------------------------------------------------------------

        let response = match redis::cmd("ZRANGE")
            .arg(get_today_key())
            .arg(0)
            .arg(-1)
            .arg("withscores")
            .query::<Vec<String>>(&mut conn)
        {
            Err(err) => {
                if let ErrorKind::TypeError = err.kind() {
                    return Ok(FetchResponseKind::NotFound(Some(token.token)));
                };

                return Err(creation_err(
                    format!(
                        "Unexpected error detected on retrieve record: {err}"
                    ),
                    None,
                    None,
                ));
            }
            Ok(res) => {
                warn!("res: {:?}", res);
                res
            }
        };

        debug!("response: {:?}", response);

        let target_key: Vec<Token> = response
            .chunks(2)
            .into_iter()
            .filter_map(|val| {
                //
                // Check if this section has enough arguments to work. Case not
                // return a None option.
                //
                if val.len() != 2 {
                    return None;
                }
                //
                // Try to extract id and service name from the redis response.
                // Case if is not possible return a None option.
                //
                let (id, svc) = match from_redis_key(val[0].to_owned()) {
                    Err(_) => return None,
                    Ok(res) => res,
                };
                //
                // Try to extract the expiration time. Case if is not possible
                // return a None option.
                //
                let ts = match NaiveDateTime::from_timestamp_opt(
                    match val[1].parse::<i64>() {
                        Err(err) => {
                            warn!("Error on parse redis key: {err}");
                            return None;
                        }
                        Ok(res) => res,
                    },
                    0,
                ) {
                    None => return None,
                    Some(res) => {
                        let utc: DateTime<Utc> = DateTime::from_utc(res, Utc);
                        let dt_local: DateTime<Local> = DateTime::from(utc);
                        dt_local
                    }
                };
                //
                // Check if all collected information are compatible with
                // desired token and it was not expired. Case true, return the
                // Some response containing the Token.
                //
                if (id == token.token) &&
                    (svc == token.own_service) &&
                    (ts >= Local::now())
                {
                    return Some(Token {
                        token: id,
                        own_service: svc,
                    });
                };
                //
                // Case the previous check was not possible a None option.
                //
                None
            })
            .collect();

        // If the target_key search returns just one record the deregistration
        // process should considered done and the token should be considered
        // found.
        if target_key.len() == 1 {
            return Ok(FetchResponseKind::Found(token));
        };

        // Otherwise, the default response if not-found.
        Ok(FetchResponseKind::NotFound(Some(token.token)))
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use log::{error, info};
    use test_log::test;
    use uuid::Uuid;

    #[test(tokio::test)]
    async fn test_token_deregistration_works() {
        let repo = TokenDeregistrationMemDbRepository {};

        match repo
            .get_then_delete(Token {
                token: Uuid::from_str("a809906e-2f6c-47f4-88d0-b51722a57a5e")
                    .unwrap(),
                own_service: String::from("some service"),
            })
            .await
        {
            Err(err) => error!("test err: {err}"),
            Ok(res) => info!("test res: {:?}", res),
        };
    }
}
