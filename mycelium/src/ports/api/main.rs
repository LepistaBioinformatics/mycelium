use std::env::set_var;

#[tokio::main]
pub async fn main() {
    // Build logger
    set_var("RUST_LOG", "debug");
    env_logger::init();
}
