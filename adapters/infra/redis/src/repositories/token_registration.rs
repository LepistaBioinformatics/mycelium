use super::functions::to_redis_key;
use crate::repositories::connector::get_connection;

use async_trait::async_trait;
use chrono::{Duration, Local};
use clean_base::{
    entities::default_response::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::{dtos::token::TokenDTO, entities::TokenRegistration};
use shaku::Component;
use std::collections::HashMap;

#[derive(Component)]
#[shaku(interface = TokenRegistration)]
pub struct TokenRegistrationSqlDbRepository {}

#[async_trait]
impl TokenRegistration for TokenRegistrationSqlDbRepository {
    async fn create(
        &self,
        token: TokenDTO,
    ) -> Result<CreateResponseKind<TokenDTO>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build connection
        // ? -------------------------------------------------------------------

        let mut conn = get_connection().await;

        // ? -------------------------------------------------------------------
        // ? Try to build insert data
        // ? -------------------------------------------------------------------

        let mut data = HashMap::new();

        data.insert(
            "expires",
            match token.to_owned().expires {
                None => (Local::now() + Duration::seconds(5)).to_string(),
                Some(res) => res.to_string(),
            },
        );

        data.insert("own_service", token.to_owned().own_service);

        // ? -------------------------------------------------------------------
        // ? Try to persist data
        // ? -------------------------------------------------------------------

        match redis::cmd("SET")
            .arg(to_redis_key(token.to_owned()))
            .arg(serde_json::to_string(&data).unwrap())
            .query::<()>(&mut conn)
        {
            Err(err) => {
                return Err(creation_err(
                    format!(
                        "Unexpected error detected on create record: {err}"
                    ),
                    None,
                    None,
                ))
            }
            Ok(_) => Ok(CreateResponseKind::Created(token)),
        }
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use log::{error, info};
    use test_log::test;
    use uuid::Uuid;

    #[test(tokio::test)]
    async fn test_token_registration_works() {
        let repo = TokenRegistrationSqlDbRepository {};

        match repo
            .create(TokenDTO {
                token: Uuid::new_v4(),
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
