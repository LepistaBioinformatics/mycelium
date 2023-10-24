use clean_base::utils::errors::{factories::execution_err, MappedErrors};
use deadpool_redis::Pool;
use futures::lock::{Mutex, MutexGuard};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::process::id as process_id;

pub fn initialize_redis_pool_() -> Result<Pool, MappedErrors> {
    let redis_url =
        std::env::var("REDIS_URL").expect("Failed to get REDIS_URL.");

    let cfg = deadpool_redis::Config::from_url(redis_url.clone());

    match cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1)) {
        Ok(pool) => Ok(pool),
        Err(e) => execution_err(format!("Failed to create Redis pool: {}", e))
            .as_error(),
    }
}

lazy_static! {
    #[derive(Debug)]
    pub(super) static ref REDIS_CONNECTOR: Pool = match initialize_redis_pool_() {
        Ok(pool) => pool,
        Err(e) => panic!("Failed to create Redis pool: {}", e),
    };
}

lazy_static! {
    #[derive(Debug)]
    //pub(crate) static ref REDIS_CONNECTION_POOL: Mutex<Option<Pool>> = Mutex::new(None);
    static ref REDIS_CONNECTION_POOL: Mutex<HashMap<u32, Pool>> = Mutex::new(HashMap::new());
}

pub async fn generate_redis_connection_pool_of_thread(
    redis_url: String,
    current_thread_id: u32,
) {
    let mut tmp_client = REDIS_CONNECTION_POOL.lock().await;

    if !tmp_client.contains_key(&current_thread_id) {
        tmp_client.insert(
            current_thread_id,
            match deadpool_redis::Config::from_url(redis_url.clone())
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            {
                Ok(pool) => pool,
                Err(e) => {
                    panic!("Failed to create Redis pool: {e}")
                }
            },
        );
    };
}

/// Get the prisma client that matches the current PID.
pub async fn get_client(
    redis_url: String,
) -> MutexGuard<'static, HashMap<u32, Pool>> {
    generate_redis_connection_pool_of_thread(redis_url, process_id()).await;
    REDIS_CONNECTION_POOL.lock().await
}
