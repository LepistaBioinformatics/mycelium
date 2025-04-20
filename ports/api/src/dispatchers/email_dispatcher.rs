use myc_core::domain::entities::{
    LocalMessageReading, LocalMessageWrite, RemoteMessageWrite,
};
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
    local_message_read_repo: Arc<dyn LocalMessageReading>,
    local_message_write_repo: Arc<dyn LocalMessageWrite>,
    remote_message_write_repo: Arc<dyn RemoteMessageWrite>,
) {
    tokio::spawn(async move {
        tracing::info!("Starting email dispatcher");

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
                local_message_read_repo.clone(),
                local_message_write_repo.clone(),
                remote_message_write_repo.clone(),
            )
            .await
            {
                Ok((messages_success, messages_failed)) => {
                    if messages_success > 0 {
                        tracing::info!(
                            "'{}' messages consumed from the queue '{}'",
                            messages_success,
                            queue_name.to_owned()
                        );
                    }

                    if messages_failed > 0 {
                        tracing::error!(
                            "'{}' messages failed to be consumed from the queue '{}'",
                            messages_failed,
                            queue_name.to_owned()
                        );
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
