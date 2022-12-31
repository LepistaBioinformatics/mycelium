use super::{functions::to_redis_key, models::TmpTokenDTO};
use crate::repositories::connector::get_connection;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::{dtos::token::TokenDTO, entities::TokenDeregistration};
use redis::ErrorKind;
use shaku::Component;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenDeregistration)]
pub struct TokenDeregistrationSqlDbRepository {}

#[async_trait]
impl TokenDeregistration for TokenDeregistrationSqlDbRepository {
    async fn get_then_delete(
        &self,
        token: TokenDTO,
    ) -> Result<FetchResponseKind<TokenDTO, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build connection
        // ? -------------------------------------------------------------------

        let mut conn = get_connection().await;

        // ? -------------------------------------------------------------------
        // ? Try to persist data
        // ? -------------------------------------------------------------------

        match redis::cmd("GETDEL")
            .arg(to_redis_key(token.to_owned()))
            .query::<TmpTokenDTO>(&mut conn)
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
            Ok(res) => Ok(FetchResponseKind::Found(TokenDTO {
                token: token.token,
                expires: res.expires,
                own_service: res.own_service,
            })),
        }
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
        let repo = TokenDeregistrationSqlDbRepository {};

        match repo
            .get_then_delete(TokenDTO {
                token: Uuid::from_str("01fce10a-5c83-4084-8e51-366818a2f20f")
                    .unwrap(),
                expires: None,
                own_service: String::from("some service"),
            })
            .await
        {
            Err(err) => error!("test err: {err}"),
            Ok(res) => info!("test res: {:?}", res),
        };
    }
}
