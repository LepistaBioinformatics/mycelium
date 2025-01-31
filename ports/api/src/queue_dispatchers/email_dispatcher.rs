use myc_config::optional_config::OptionalConfig;
use myc_notifier::{
    executor::consume_messages, models::QueueConfig,
    repositories::MessageSendingSmtpRepository,
};
use std::time::Duration;

/// Dispatch email messages
///
/// Spawns a new thread to consume messages from the email queue.
///
#[tracing::instrument(name = "email_dispatcher", skip_all)]
pub(crate) fn email_dispatcher(queue: OptionalConfig<QueueConfig>) {
    let queue_config = match queue.to_owned() {
        OptionalConfig::Enabled(queue) => queue,
        _ => panic!("Queue config not found"),
    };

    tokio::spawn(async move {
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
                // TODO: Remove this once the repository is implemented as a
                // shaku module
                Box::new(&MessageSendingSmtpRepository {}),
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
