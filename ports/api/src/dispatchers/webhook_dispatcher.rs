use crate::models::active_backend_modules::SqlAppModule;
use futures::future::join_all;
use myc_core::domain::dtos::webhook::{
    WebHookExecutionStatus, WebHookRetryPolicy,
};
use myc_core::domain::entities::WebHookUpdating;
use myc_core::models::CoreConfig;
use myc_core::{
    domain::entities::{EncryptionKeyFetching, WebHookFetching},
    use_cases::dispatch_webhooks,
};
use mycelium_base::entities::FetchManyResponseKind;
use rand::Rng;
use shaku::HasComponent;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Dispatch webhooks
///
/// Spawns a new thread to consume messages from the webhook queue.
///
#[tracing::instrument(name = "webhook_dispatcher", skip_all)]
pub(crate) async fn webhook_dispatcher(
    config: CoreConfig,
    app_modules: Arc<SqlAppModule>,
) {
    tokio::spawn(async move {
        tracing::info!("Starting webhook dispatcher");

        let webhook_config = config.webhook.clone();
        let read_repo: &dyn WebHookFetching = app_modules.resolve_ref();
        let write_repo: &dyn WebHookUpdating = app_modules.resolve_ref();
        let enc_key_repo: &dyn EncryptionKeyFetching =
            app_modules.resolve_ref();
        let child_read_repo = Box::new(read_repo);
        let child_write_repo = Box::new(write_repo);
        let child_enc_key_repo = Box::new(enc_key_repo);
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

        //
        // Wait for a random time between 1 and the consume interval. This only
        // staggers this dispatcher against the email one so they do not both
        // wake on the same second; it is NOT what keeps two replicas off the
        // same event. That is the repository's claim (`FOR UPDATE SKIP
        // LOCKED`), which jitter alone never provided.
        //
        let random_time =
            rand::thread_rng().gen_range(1..=interval.period().as_secs());

        tokio::time::sleep(Duration::from_secs(random_time)).await;

        loop {
            interval.tick().await;

            //
            // Fetch webhook dispatch events
            //
            let retry_policy = WebHookRetryPolicy::new(
                webhook_config
                    .retry_base_in_secs
                    .async_get_or_error()
                    .await
                    .unwrap_or(30),
                webhook_config
                    .retry_cap_in_secs
                    .async_get_or_error()
                    .await
                    .unwrap_or(3600),
                webhook_config
                    .visibility_timeout_in_secs
                    .async_get_or_error()
                    .await
                    .unwrap_or(900),
            );

            //
            // `Processing` is deliberately absent from this filter. A row a
            // live pod is working on must not be handed out again, and one left
            // behind by a pod that died is picked up by the repository's own
            // stale-claim branch, which keys off the lease clock rather than
            // off this list.
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
                    retry_policy,
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
                continue;
            }

            //
            // Dispatch webhooks
            //
            for ((trigger, id), artifacts) in events_by_trigger {
                tracing::info!(
                    "Dispatch webhooks for trigger {trigger} and id {id}: {artifacts}",
                    trigger = trigger,
                    id = id,
                    artifacts = artifacts.len()
                );

                let dispatching_events =
                    join_all(artifacts.into_iter().map(|artifact| {
                        dispatch_webhooks(
                            trigger.to_owned(),
                            artifact,
                            config.clone(),
                            child_read_repo.clone(),
                            child_write_repo.clone(),
                            child_enc_key_repo.clone(),
                        )
                    }))
                    .await;

                for event in dispatching_events {
                    if let Err(err) = event {
                        tracing::error!("Error on dispatch webhook: {err}");
                    }
                }
            }
        }
    });
}
