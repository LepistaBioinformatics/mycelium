use clean_base::utils::errors::{factories::execution_err, MappedErrors};
use deadpool_redis::Pool;
use lazy_static::lazy_static;

pub fn generate_redis_pool() -> Result<Pool, MappedErrors> {
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
    pub(super) static ref REDIS_CONNECTOR: Pool = match generate_redis_pool() {
        Ok(pool) => pool,
        Err(e) => panic!("Failed to create Redis pool: {}", e),
    };
}
