use crate::models::QueueConfig;

use lazy_static::lazy_static;
use mycelium_base::utils::errors::MappedErrors;
use redis::Client;
use std::sync::Mutex;

lazy_static! {
    #[derive(Debug)]
    pub(super) static ref QUEUE_CLIENT: Mutex<Option<Client>> = Mutex::new(None);
}

/// Initialize the queue client from a given URL
///
/// This function should be used to initialize the queue client on the
/// application startup.
pub async fn init_queue_client_from_url(
    config: QueueConfig,
) -> Result<(), MappedErrors> {
    let url = format!(
        "{}://:{}@{}",
        config.protocol,
        config.password.get_or_error()?,
        config.hostname.get_or_error()?
    );

    QUEUE_CLIENT
        .lock()
        .unwrap()
        .replace(Client::open(url).unwrap());

    Ok(())
}

/// Get the queue client
///
/// This function should be used to get the queue client instance from the
/// application.
pub(super) async fn get_client() -> Client {
    QUEUE_CLIENT
        .lock()
        .expect("Could not connect to the queue")
        .as_ref()
        .expect("Queue client not initialized")
        .to_owned()
}
