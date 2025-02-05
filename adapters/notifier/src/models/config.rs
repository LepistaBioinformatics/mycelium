use super::QueueConfig;

use lettre::SmtpTransport;
use myc_adapters_shared_lib::models::RedisConfig;
use redis::Client;
use shaku::Interface;
use std::sync::Arc;

pub trait ClientProvider: Interface + Send + Sync {
    fn get_queue_config(&self) -> Arc<QueueConfig>;
    fn get_smtp_client(&self) -> Arc<SmtpTransport>;
    fn get_redis_config(&self) -> Arc<RedisConfig>;
    fn get_redis_client(&self) -> Arc<Client>;
}
