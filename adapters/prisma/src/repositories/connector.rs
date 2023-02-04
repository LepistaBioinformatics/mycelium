use crate::prisma::PrismaClient;
use futures::lock::{Mutex, MutexGuard};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::process::id as process_id;

lazy_static! {
    #[derive(Debug)]
    pub static ref PRIMA_CONNECTOR: Mutex<HashMap<u32, PrismaClient>> = Mutex::new(HashMap::new());
}

/// This function check it the current thread already contains a prisma client
/// registered. Case not, create a new client and store it into the map that
/// includes the thread ID as a key and the prisma client instance as a value.
pub async fn generate_prisma_client_of_thread(current_thread_id: u32) {
    let mut tmp_client = PRIMA_CONNECTOR.lock().await;

    if !tmp_client.contains_key(&current_thread_id) {
        tmp_client.insert(
            current_thread_id,
            match PrismaClient::_builder().build().await {
                Ok(conn) => conn,
                Err(err) => {
                    panic!(
                        "Error detected on initialize prisma client: {}",
                        err
                    )
                }
            },
        );
    };
}

/// Get the prisma client that matches the current PID.
pub async fn get_client() -> MutexGuard<'static, HashMap<u32, PrismaClient>> {
    generate_prisma_client_of_thread(process_id()).await;
    PRIMA_CONNECTOR.lock().await
}
