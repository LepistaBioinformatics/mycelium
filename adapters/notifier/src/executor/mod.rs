use tracing::debug;

#[tracing::instrument(name = "consume_messages")]
pub async fn consume_messages() -> String {
    debug!("consume_messages");

    "consume_messages".to_string()
}
