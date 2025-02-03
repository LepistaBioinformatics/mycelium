use futures::future::join_all;
use myc_core::domain::dtos::webhook::WebHookExecutionStatus;
use myc_core::domain::entities::WebHookUpdating;
use myc_core::models::CoreConfig;
use myc_core::{
    domain::entities::WebHookFetching, use_cases::dispatch_webhooks,
};
use myc_diesel::repositories::SqlAppModule;
use mycelium_base::entities::FetchManyResponseKind;
use shaku::HasComponent;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Dispatch webhooks
///
/// Spawns a new thread to consume messages from the webhook queue.
///
#[tracing::instrument(name = "webhook_dispatcher", skip_all)]
pub(crate) fn webhook_dispatcher(
    config: CoreConfig,
    app_modules: Arc<SqlAppModule>,
) {
    tokio::spawn(async move {
        let webhook_config = config.webhook.clone();
        let read_repo: &dyn WebHookFetching = app_modules.resolve_ref();
        let write_repo: &dyn WebHookUpdating = app_modules.resolve_ref();
        let child_read_repo = Box::new(read_repo);
        let child_write_repo = Box::new(write_repo);
        let mut interval =
            actix_rt::time::interval(Duration::from_secs(match webhook_config
                .consume_interval_in_secs
                .async_get_or_error()
                .await
            {
                Ok(interval) => interval,
                Err(err) => {
                    panic!("Error on get consume interval: {err}");
                }
            }));

        //
        // Skip the first tick to avoid fetching events that were created in the
        // same second as the dispatcher start.
        //
        interval.tick().await;

        loop {
            interval.tick().await;

            tracing::trace!("Start webhook dispatch");

            //
            // Fetch webhook dispatch events
            //
            let events_response = match read_repo
                .fetch_execution_event(
                    webhook_config
                        .consume_batch_size
                        .async_get_or_error()
                        .await
                        .unwrap_or(10) as u32,
                    webhook_config
                        .max_attempts
                        .async_get_or_error()
                        .await
                        .unwrap_or(3) as u32,
                    Some(vec![
                        WebHookExecutionStatus::Pending,
                        WebHookExecutionStatus::Failed,
                    ]),
                )
                .await
            {
                Ok(events) => events,
                Err(err) => {
                    tracing::error!("Error on fetch execution event: {err}");
                    continue;
                }
            };

            let events = match events_response {
                FetchManyResponseKind::NotFound => {
                    tracing::trace!("No webhook dispatch events found");
                    continue;
                }
                FetchManyResponseKind::Found(events) => events,
                FetchManyResponseKind::FoundPaginated { records, .. } => {
                    records
                }
            };

            //
            // Fold events by trigger
            //
            let events_by_trigger =
                events.into_iter().fold(HashMap::new(), |mut acc, event| {
                    let id = event.id.unwrap_or_else(|| {
                        panic!("Webhook artifact id is required");
                    });

                    acc.entry((event.trigger.clone(), id))
                        .or_insert_with(Vec::new)
                        .push(event);

                    acc
                });

            if events_by_trigger.is_empty() {
                tracing::trace!("No webhook dispatch events found");
                continue;
            }

            //
            // Dispatch webhooks
            //
            for ((trigger, id), artifacts) in events_by_trigger {
                tracing::info!(
                    "Dispatch webhooks for trigger: {trigger} and id: {id}"
                );

                let dispatching_events =
                    join_all(artifacts.into_iter().map(|artifact| {
                        dispatch_webhooks(
                            trigger.to_owned(),
                            artifact,
                            config.clone(),
                            child_read_repo.clone(),
                            child_write_repo.clone(),
                        )
                    }))
                    .await;

                for event in dispatching_events {
                    if let Err(err) = event {
                        tracing::error!("Error on dispatch webhook: {err}");
                    }
                }
            }

            tracing::trace!("End webhooks dispatch");
        }
    });
}
