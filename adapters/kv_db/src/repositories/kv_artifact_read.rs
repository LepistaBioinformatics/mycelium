use async_trait::async_trait;
use myc_adapters_shared_lib::models::SharedClientProvider;
use myc_core::domain::entities::KVArtifactRead;
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use redis::{Commands, FromRedisValue, Value};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = KVArtifactRead)]
pub struct KVArtifactReadRepository {
    #[shaku(inject)]
    client: Arc<dyn SharedClientProvider>,
}

#[async_trait]
impl KVArtifactRead for KVArtifactReadRepository {
    #[tracing::instrument(name = "get_encoded_artifact", skip_all)]
    async fn get_encoded_artifact(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<String, String>, MappedErrors> {
        let mut connection = self
            .client
            .get_redis_client()
            .as_ref()
            .clone()
            .get_connection()
            .map_err(|err| {
                tracing::error!("Error on get redis connection: {err}");

                fetching_err("Error on get redis connection")
            })?;

        let rv: Value = connection.get(key.to_owned()).map_err(|err| {
            tracing::error!("Error on get redis artifact: {err}");

            fetching_err("Error on get redis artifact")
        })?;

        let message_str: String = if Value::Nil == rv {
            return Ok(FetchResponseKind::NotFound(Some(key)));
        } else {
            let parsed_value = FromRedisValue::from_redis_value(&rv);

            match parsed_value {
                Ok(val) => val,
                Err(err) => {
                    return fetching_err(format!(
                        "Failed to parse notification from the message queue: {err}"
                    ))
                    .as_error()
                }
            }
        };

        Ok(FetchResponseKind::Found(message_str))
    }
}
