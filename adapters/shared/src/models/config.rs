use super::RedisConfig;

use mycelium_base::utils::errors::{execution_err, MappedErrors};
use redis::Client;
use shaku::{Component, Interface};
use std::sync::Arc;

pub trait SharedClientProvider: Interface + Send + Sync {
    fn get_redis_client(&self) -> Arc<Client>;
    fn get_redis_config(&self) -> Arc<RedisConfig>;
}

#[derive(Component)]
#[shaku(interface = SharedClientProvider)]
#[derive(Clone)]
pub struct SharedClientImpl {
    redis_client: Arc<Client>,
    redis_config: Arc<RedisConfig>,
}

impl SharedClientProvider for SharedClientImpl {
    fn get_redis_client(&self) -> Arc<Client> {
        self.redis_client.clone()
    }

    fn get_redis_config(&self) -> Arc<RedisConfig> {
        self.redis_config.clone()
    }
}

impl SharedClientImpl {
    pub async fn new(
        redis_config: RedisConfig,
    ) -> Result<Arc<Self>, MappedErrors> {
        let redis_url = format!(
            "{}://:{}@{}:{}",
            redis_config.protocol.async_get_or_error().await?,
            redis_config.password.async_get_or_error().await?,
            redis_config.hostname.async_get_or_error().await?,
            match redis_config.to_owned().port {
                Some(port) => port.async_get_or_error().await?,
                None => 6379,
            }
        );

        let client = Client::open(redis_url).map_err(|err| {
            execution_err(format!("Failed to connect to redis: {err}"))
        })?;

        Ok(Arc::new(Self {
            redis_client: Arc::new(client),
            redis_config: Arc::new(redis_config),
        }))
    }
}
