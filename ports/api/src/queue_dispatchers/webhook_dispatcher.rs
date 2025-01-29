/// Dispatch webhooks
///
/// Spawns a new thread to consume messages from the webhook queue.
///
#[tracing::instrument(name = "webhook_dispatcher", skip_all)]
pub(crate) fn webhook_dispatcher() {
    unimplemented!("Webhook dispatcher not implemented");
}
