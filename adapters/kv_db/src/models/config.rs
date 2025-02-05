use super::QueueConfig;

use lettre::SmtpTransport;
use redis::Client;
use shaku::Interface;
use std::sync::Arc;

pub trait ClientProvider: Interface + Send + Sync {
    fn get_queue_client(&self) -> Arc<Client>;
    fn get_smtp_client(&self) -> Arc<SmtpTransport>;
    fn get_config(&self) -> Arc<QueueConfig>;
}
