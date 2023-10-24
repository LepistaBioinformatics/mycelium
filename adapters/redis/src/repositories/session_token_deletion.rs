use super::connectors::get_client;
use std::process::id as process_id;

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
        let tmp_client = get_client("url".to_string()).await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .as_error()
            }
            Some(res) => res,
        };

        let mut connection = match client.get().await {
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
