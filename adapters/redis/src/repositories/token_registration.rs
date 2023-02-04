use super::{
    connector::get_connection,
    functions::{get_today_key, to_redis_key},
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use clean_base::{
    entities::default_response::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::{dtos::token::Token, entities::TokenRegistration};
use shaku::Component;

#[derive(Component)]
#[shaku(interface = TokenRegistration)]
pub struct TokenRegistrationMemDbRepository {}

#[async_trait]
impl TokenRegistration for TokenRegistrationMemDbRepository {
    async fn create(
        &self,
        token: Token,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build connection
        // ? -------------------------------------------------------------------

        let mut conn = get_connection().await;

        // ? -------------------------------------------------------------------
        // ? Try to persist data
        // ? -------------------------------------------------------------------

        match redis::cmd("ZADD")
            .arg(get_today_key())
            .arg(expires.timestamp())
            .arg(to_redis_key(token.to_owned()))
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
    use myc_core::use_cases::roles::service::token::generate_token_expiration_time;
    use test_log::test;
    use uuid::Uuid;

    #[test(tokio::test)]
    async fn test_token_registration_works() {
        let repo = TokenRegistrationMemDbRepository {};

        match repo
            .create(
                Token {
                    token: Uuid::new_v4(),
                    own_service: String::from("some service"),
                },
                generate_token_expiration_time().await,
            )
            .await
        {
            Err(err) => error!("test err: {err}"),
            Ok(res) => info!("test res: {:?}", res),
        };
    }
}
