use futures::lock::{Mutex, MutexGuard};
use lazy_static::lazy_static;
use log::info;
use redis::{Client, Connection};
use std::env::var_os;

lazy_static! {

    /// Consensus redis connection string is "redis://127.0.0.1:6378/"
    #[derive(Debug)]
    pub static ref REDIS_CONNECTOR: Mutex<Connection> = match var_os("REDIS_URL") {
        None => panic!("`REDIS_URL` connection string not configured"),
        Some(res) => match Client::open(res.to_str().unwrap()) {
            Err(err) => panic!("Unable to connect to redis client: {err}"),
            Ok(res) => match res.get_connection() {
                Err(err) => panic!("Could not connect to redis client: {err}"),
                Ok(res) => {
                    info!("Redis connection successful established.");
                    Mutex::new(res)
                },
            },
        }
    };
}

pub async fn get_connection() -> MutexGuard<'static, Connection> {
    REDIS_CONNECTOR.lock().await
}
