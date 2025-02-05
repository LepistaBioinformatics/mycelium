use async_trait::async_trait;
use myc_adapters_shared_lib::models::SharedClientProvider;
use myc_core::domain::entities::KVArtifactWrite;
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use redis::Commands;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = KVArtifactWrite)]
pub struct KVArtifactWriteRepository {
    #[shaku(inject)]
    client: Arc<dyn SharedClientProvider>,
}

#[async_trait]
impl KVArtifactWrite for KVArtifactWriteRepository {
    #[tracing::instrument(name = "set_encoded_artifact", skip_all)]
    async fn set_encoded_artifact(
        &self,
        key: String,
        value: String,
        ttl: u64,
    ) -> Result<CreateResponseKind<String>, MappedErrors> {
        let mut connection = self
            .client
            .get_redis_client()
            .as_ref()
            .clone()
            .get_connection()
            .map_err(|err| {
                tracing::error!("Error on get redis connection: {err}");

                creation_err("Error on get redis connection")
            })?;

        let _: () =
            connection
                .set_ex(key, value.to_owned(), ttl)
                .map_err(|err| {
                    tracing::error!("Error on set redis artifact: {err}");

                    creation_err("Error on set redis artifact")
                })?;

        Ok(CreateResponseKind::Created(value))
    }
}
