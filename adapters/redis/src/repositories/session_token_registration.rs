use super::connectors::REDIS_CONNECTOR;

use async_trait::async_trait;
use clean_base::{
    entities::CreateResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use deadpool_redis::redis::cmd;
use myc_core::domain::entities::SessionTokenRegistration;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = SessionTokenRegistration)]
pub struct SessionTokenRegistrationRedisDbRepository {}

#[async_trait]
impl SessionTokenRegistration for SessionTokenRegistrationRedisDbRepository {
    async fn create(
        &self,
        session_key: String,
        session_value: String,
    ) -> Result<CreateResponseKind<bool>, MappedErrors> {
        let mut connection = match REDIS_CONNECTOR.get().await {
            Ok(conn) => conn,
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error on fetch redis connection: {err}"
                ))
                .as_error();
            }
        };

        match cmd("SET")
            .arg(&[session_key, session_value])
            .query_async::<_, ()>(&mut *connection)
            .await
        {
            Ok(_) => Ok(CreateResponseKind::Created(true)),
            Err(err) => creation_err(format!(
                "Unexpected error on set session key: {err}"
            ))
            .as_error(),
        }
    }
}
