use myc_adapters_shared_lib::models::SharedClientProvider;
use myc_core::domain::entities::{LocalMessageSending, RemoteMessageSending};
use myc_notifier::{executor::consume_messages, models::QueueConfig};
use rand::Rng;
use std::{sync::Arc, time::Duration};

/// Dispatch email messages
///
/// Spawns a new thread to consume messages from the email queue.
///
#[tracing::instrument(name = "email_dispatcher", skip_all)]
pub(crate) fn email_dispatcher(
    queue_config: QueueConfig,
    client: Arc<dyn SharedClientProvider>,
    queue_sending_repo: Arc<dyn LocalMessageSending>,
    message_sending_repo: Arc<dyn RemoteMessageSending>,
) {
    tokio::spawn(async move {
        tracing::trace!("Starting email dispatcher");

        //
        // Test local message sending connection
        //
        match queue_sending_repo.ping().await {
            Ok(_) => {
                tracing::info!("Local message sending connection is OK");
            }
            Err(err) => {
                panic!("Error on ping local message sending: {err}");
            }
        };

        let mut interval = actix_rt::time::interval(Duration::from_secs(
            match queue_config
                .consume_interval_in_secs
                .async_get_or_error()
                .await
            {
                Ok(interval) => interval,
                Err(err) => {
                    panic!("Error on get consume interval: {err}");
                }
            },
        ));

        //
        // Skip the first tick to avoid fetching events that were created in the
        // same second as the dispatcher start.
        //
        interval.tick().await;

        //
        // Wait for a random time between 1 and the consume interval. This time
        // should avoid the webhook dispatcher to start at the same time as the
        // email dispatcher and avoid the simultaneous consumption of the same
        // event over multiple containers.
        //
        let random_time =
            rand::thread_rng().gen_range(1..=interval.period().as_secs());

        tokio::time::sleep(Duration::from_secs(random_time)).await;

        loop {
            interval.tick().await;

            let queue_name = match queue_config
                .clone()
                .email_queue_name
                .async_get_or_error()
                .await
            {
                Ok(name) => name,
                Err(err) => {
                    panic!("Error on get queue name: {err}");
                }
            };

            match consume_messages(
                queue_name.to_owned(),
                client.clone(),
                message_sending_repo.clone(),
            )
            .await
            {
                Ok(messages) => {
                    if messages > 0 {
                        tracing::info!(
                            "'{}' messages consumed from the queue '{}'",
                            messages,
                            queue_name.to_owned()
                        )
                    }
                }
                Err(err) => {
                    if !err.expected() {
                        panic!("Error on consume messages: {err}");
                    }
                }
            };
        }
    });
}
