use super::QueueConfig;

use lettre::SmtpTransport;
use redis::Client;
use shaku::Interface;
use std::sync::Arc;

pub trait ClientProvider: Interface + Send + Sync {
    fn get_redis_client(&self) -> Arc<Client>;
    fn get_queue_config(&self) -> Arc<QueueConfig>;
    fn get_smtp_client(&self) -> Arc<SmtpTransport>;
}
