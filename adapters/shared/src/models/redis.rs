use super::SharedClientProvider;
use crate::models::RedisConfig;

use redis::Client;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = SharedClientProvider)]
pub struct RedisClientWrapper {
    client: Client,
    config: RedisConfig,
}

impl RedisClientWrapper {
    pub fn new(client: Client, config: RedisConfig) -> Self {
        Self { client, config }
    }
}

impl SharedClientProvider for RedisClientWrapper {
    fn get_redis_config(&self) -> Arc<RedisConfig> {
        Arc::new(self.config.clone())
    }

    fn get_redis_client(&self) -> Arc<Client> {
        Arc::new(self.client.clone())
    }
}
