use super::RedisConfig;

use redis::Client;
use shaku::Interface;
use std::sync::Arc;

pub trait ClientProvider: Interface + Send + Sync {
    fn get_queue_client(&self) -> Arc<Client>;
    fn get_config(&self) -> Arc<RedisConfig>;
}
