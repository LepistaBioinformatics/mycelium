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
        config.password.async_get_or_error().await?,
        config.hostname.async_get_or_error().await?
    );

    QUEUE_CLIENT
        .lock()
        .expect("Could not fix the queue config")
        .replace(Client::open(url).expect("Could not connect to the queue"));

    Ok(())
}

/// Get the queue client
///
/// This function should be used to get the queue client instance from the
/// application.
pub(crate) async fn get_client() -> Client {
    match QUEUE_CLIENT
        .lock()
        .expect("Could not connect to the queue")
        .as_ref()
    {
        Some(client) => client.clone(),
        None => panic!("Queue client is not initialized"),
    }
}
