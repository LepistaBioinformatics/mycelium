use super::RedisConfig;

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
pub struct SharedClientImpl;

impl SharedClientProvider for SharedClientImpl {
    fn get_redis_client(&self) -> Arc<Client> {
        todo!()
    }

    fn get_redis_config(&self) -> Arc<RedisConfig> {
        todo!()
    }
}
