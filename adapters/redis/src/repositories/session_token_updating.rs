use super::connectors::REDIS_CONNECTOR;

use async_trait::async_trait;
use chrono::Duration;
use clean_base::{
    entities::UpdatingResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use deadpool_redis::redis::cmd;
use myc_core::domain::entities::SessionTokenUpdating;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = SessionTokenUpdating)]
pub struct SessionTokenUpdatingRedisDbRepository {}

#[async_trait]
impl SessionTokenUpdating for SessionTokenUpdatingRedisDbRepository {
    async fn update(
        &self,
        session_key: String,
        time_to_live: Duration,
    ) -> Result<UpdatingResponseKind<bool>, MappedErrors> {
        let mut connection = match REDIS_CONNECTOR.get().await {
            Ok(conn) => conn,
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error on fetch redis connection: {err}"
                ))
                .as_error();
            }
        };

        match cmd("EXPIRE")
            .arg(&[session_key, time_to_live.num_seconds().to_string()])
            .query_async::<_, ()>(&mut *connection)
            .await
        {
            Ok(_) => Ok(UpdatingResponseKind::Updated(true)),
            Err(err) => creation_err(format!(
                "Unexpected error on set expires session key: {err}"
            ))
            .as_error(),
        }
    }
}
