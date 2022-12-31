use futures::lock::{Mutex, MutexGuard};
use lazy_static::lazy_static;
use redis::{Client, Connection};

lazy_static! {
    #[derive(Debug)]
    pub static ref REDIS_CONNECTOR: Mutex<Connection> = Mutex::new(match Client::open("redis://127.0.0.1:6378/") {
        Err(err) => panic!("Unable to connect to redis client: {err}"),
        Ok(res) => match res.get_connection() {
            Err(err) => panic!("Could not connect to redis client: {err}"),
            Ok(res) => res,
        },
    });
}

pub async fn get_connection() -> MutexGuard<'static, Connection> {
    REDIS_CONNECTOR.lock().await
}
