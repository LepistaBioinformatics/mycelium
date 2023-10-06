use super::connectors::REDIS_CONNECTOR;

use async_trait::async_trait;
use clean_base::{
    entities::FetchResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use deadpool_redis::redis::cmd;
use myc_core::domain::entities::SessionTokenFetching;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = SessionTokenFetching)]
pub struct SessionTokenFetchingRedisDbRepository {}

#[async_trait]
impl SessionTokenFetching for SessionTokenFetchingRedisDbRepository {
    async fn get(
        &self,
        session_key: String,
    ) -> Result<FetchResponseKind<String, String>, MappedErrors> {
        let mut connection = match REDIS_CONNECTOR.get().await {
            Ok(conn) => conn,
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error on fetch redis connection: {err}"
                ))
                .as_error();
            }
        };

        match cmd("GET")
            .arg(&[session_key])
            .query_async(&mut *connection)
            .await
        {
            Ok(res) => Ok(FetchResponseKind::Found(res)),
            Err(err) => creation_err(format!(
                "Unexpected error on get session key: {err}"
            ))
            .as_error(),
        }
    }
}
