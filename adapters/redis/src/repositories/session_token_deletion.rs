use super::connectors::REDIS_CONNECTOR;

use async_trait::async_trait;
use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use deadpool_redis::redis::cmd;
use myc_core::domain::entities::SessionTokenDeletion;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = SessionTokenDeletion)]
pub struct SessionTokenDeletionRedisDbRepository {}

#[async_trait]
impl SessionTokenDeletion for SessionTokenDeletionRedisDbRepository {
    async fn delete(
        &self,
        session_key: String,
    ) -> Result<DeletionResponseKind<String>, MappedErrors> {
        let mut connection = match REDIS_CONNECTOR.get().await {
            Ok(conn) => conn,
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error on fetch redis connection: {err}"
                ))
                .as_error();
            }
        };

        match cmd("DEL")
            .arg(&[session_key])
            .query_async::<_, ()>(&mut *connection)
            .await
        {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(err) => creation_err(format!(
                "Unexpected error on get session key: {err}"
            ))
            .as_error(),
        }
    }
}
