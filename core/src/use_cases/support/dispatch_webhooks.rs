use crate::{
    domain::{
        dtos::{
            http::HttpMethod,
            resolved_http_secret::ResolvedHttpSecret,
            webhook::{
                HookResponse, WebHook, WebHookExecutionStatus,
                WebHookPayloadArtifact, WebHookTrigger,
            },
        },
        entities::{EncryptionKeyFetching, WebHookFetching, WebHookUpdating},
        utils::{build_aad, AAD_FIELD_HTTP_SECRET},
    },
    models::CoreConfig,
};

use chrono::Local;
use futures_util::future::join_all;
use mycelium_base::{
    entities::{FetchManyResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use reqwest::Client;
use std::time::Duration;

#[tracing::instrument(
    name = "dispatch_webhooks",
    fields(trigger = %trigger, artifact_id = %artifact.id.unwrap_or_default()),
    skip(config, artifact, webhook_fetching_repo, webhook_updating_repo,
         encryption_key_fetching_repo)
)]
pub async fn dispatch_webhooks(
    trigger: WebHookTrigger,
    artifact: WebHookPayloadArtifact,
    config: CoreConfig,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
    webhook_updating_repo: Box<&dyn WebHookUpdating>,
    encryption_key_fetching_repo: Box<&dyn EncryptionKeyFetching>,
) -> Result<WebHookPayloadArtifact, MappedErrors> {
    // Resolved before anything else so every exit below can decide between
    // `Failed` and the terminal `Exhausted`. A failure here is a config
    // failure, not an event failure: it hits every event equally and leaves
    // the batch to be reclaimed after the visibility window.
    let max_attempts = config.webhook.max_attempts.async_get_or_error().await?;

    let updating_repo = *webhook_updating_repo;

    let mut artifact = match artifact.decode_payload() {
        Ok(decoded) => decoded,
        Err(err) => {
            return Err(abort_attempt(
                artifact,
                max_attempts,
                updating_repo,
                format!("Error on decoding webhook payload: {err}"),
            )
            .await);
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Find for webhooks that are triggered by the event
    // ? -----------------------------------------------------------------------

    let hooks_fetching_response = match webhook_fetching_repo
        .list_by_trigger(trigger.to_owned())
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return Err(abort_attempt(
                artifact,
                max_attempts,
                updating_repo,
                format!("Error on fetching webhooks: {err}"),
            )
            .await);
        }
    };

    let hooks: Vec<WebHook> = match hooks_fetching_response {
        FetchManyResponseKind::Found(records) => records,
        FetchManyResponseKind::NotFound => {
            return record_attempt(
                artifact,
                WebHookExecutionStatus::Skipped,
                max_attempts,
                updating_repo,
            )
            .await;
        }
        _ => {
            return Err(abort_attempt(
                artifact,
                max_attempts,
                updating_repo,
                "Webhook response should not be paginated".to_string(),
            )
            .await);
        }
    };

    tracing::info!("Found {} webhooks to dispatch", hooks.len());

    // ? -----------------------------------------------------------------------
    // ? Pre-fetch the system DEK once for all webhook secrets
    // ? -----------------------------------------------------------------------

    let kek = match config.account_life_cycle.derive_kek_bytes().await {
        Ok(kek) => kek,
        Err(err) => {
            return Err(abort_attempt(
                artifact,
                max_attempts,
                updating_repo,
                format!("Error on deriving the KEK: {err}"),
            )
            .await);
        }
    };

    let system_dek = match encryption_key_fetching_repo
        .get_or_provision_dek(None, &kek)
        .await
    {
        Ok(dek) => dek,
        Err(err) => {
            return Err(abort_attempt(
                artifact,
                max_attempts,
                updating_repo,
                format!("Error on fetching the system DEK: {err}"),
            )
            .await);
        }
    };

    let system_aad = build_aad(None, AAD_FIELD_HTTP_SECRET);

    // ? -----------------------------------------------------------------------
    // ? Decrypt all webhook secrets up-front (async, before the sync map)
    // ? -----------------------------------------------------------------------

    let decrypted_secrets =
        match resolve_hook_secrets(&hooks, &system_dek, &config, &system_aad)
            .await
        {
            Ok(secrets) => secrets,
            Err(err) => {
                return Err(abort_attempt(
                    artifact,
                    max_attempts,
                    updating_repo,
                    format!("Error on resolving webhook secrets: {err}"),
                )
                .await);
            }
        };

    // ? -----------------------------------------------------------------------
    // ? Build requests to the webhooks
    // ? -----------------------------------------------------------------------

    // Both timeouts are what bounds a dispatch at all: without them a
    // downstream that accepts the connection and never answers holds this
    // future open forever, and the claim's visibility window has no worst case
    // to be sized against.
    let client = match build_dispatch_client(&config).await {
        Ok(client) => client,
        Err(err) => {
            return Err(abort_attempt(
                artifact,
                max_attempts,
                updating_repo,
                format!("Error on building client: {err}"),
            )
            .await);
        }
    };

    let bodies: Vec<_> = hooks
        .iter()
        .zip(decrypted_secrets.iter())
        .map(|(hook, decrypted_secret)| {
            let client = client.clone();
            let payload = artifact.payload.to_owned();
            let artifact_id = artifact.id;

            async move {
                let method = match hook.method {
                    Some(method) => method,
                    None => HttpMethod::Post,
                };

                let base_request = match method {
                    HttpMethod::Post => client.post(hook.url.to_owned()),
                    HttpMethod::Put => client.put(hook.url.to_owned()),
                    HttpMethod::Patch => client.patch(hook.url.to_owned()),
                    HttpMethod::Delete => client.delete(match artifact_id {
                        None => hook.url.to_owned(),
                        Some(id) => {
                            format!("{}/{}", hook.url, id)
                        }
                    }),
                    _ => {
                        tracing::error!("Unknown method: {method}");
                        client.post(hook.url.to_owned())
                    }
                };

                (match decrypted_secret {
                    Some(ResolvedHttpSecret::AuthorizationHeader {
                        header_name,
                        prefix,
                        token,
                    }) => {
                        let key = header_name
                            .clone()
                            .unwrap_or_else(|| "Authorization".to_string());
                        let value = match prefix {
                            Some(p) => format!("{} {}", p, token),
                            None => token.to_owned(),
                        };
                        base_request.header(key, value)
                    }
                    Some(ResolvedHttpSecret::QueryParameter {
                        name,
                        token,
                    }) => base_request.query(&[(name, token)]),
                    None => base_request,
                })
                .body(payload)
                .header("Content-Type", "application/json")
                .send()
                .await
            }
        })
        .collect();

    tracing::info!("Sending {} webhooks", bodies.len());

    // ? -----------------------------------------------------------------------
    // ? Propagate responses
    // ? -----------------------------------------------------------------------

    let mut responses = Vec::<HookResponse>::new();
    for hook_future in join_all(bodies).await {
        let hook_res = match hook_future {
            Ok(res) => res,
            Err(err) => {
                let url = match err.url() {
                    Some(url) => url.to_string(),
                    None => "".to_string(),
                };

                tracing::error!("Error on connect to webhook: {:?}", err);

                responses.push(HookResponse {
                    url,
                    status: 500,
                    body: Some("Error on connect to webhook".to_string()),
                    datetime: Local::now(),
                });

                continue;
            }
        };

        let url = hook_res.url();
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or("");
        let port = url.port().map(|p| format!(":{}", p)).unwrap_or_default();
        let path = url.path();

        responses.push(HookResponse {
            url: format!("{}://{}{}{}", scheme, host, port, path),
            status: hook_res.status().as_u16(),
            body: hook_res.text().await.ok(),
            datetime: Local::now(),
        });
    }

    // ? -----------------------------------------------------------------------
    // ? Evaluate the status of the artifact
    // ? -----------------------------------------------------------------------

    let status = match responses.iter().any(|response| response.status >= 400) {
        true => WebHookExecutionStatus::Failed,
        false => WebHookExecutionStatus::Success,
    };

    // ? -----------------------------------------------------------------------
    // ? Update artifact with propagation responses
    // ? -----------------------------------------------------------------------

    let mut propatations = artifact.propagations.clone().unwrap_or_default();

    if !responses.is_empty() {
        propatations.append(&mut responses);
    }

    if !propatations.is_empty() {
        artifact.propagations = Some(propatations);
    }

    // ? -----------------------------------------------------------------------
    // ? Update the artifact into data store
    // ? -----------------------------------------------------------------------

    record_attempt(artifact, status, max_attempts, updating_repo).await
}

/// Decrypt and resolve the secret of every hook, in order
///
/// Extracted from the dispatch body so the caller can treat a failure here the
/// same way it treats any other aborted attempt, instead of returning without
/// ever recording one.
///
async fn resolve_hook_secrets(
    hooks: &[WebHook],
    system_dek: &[u8; 32],
    config: &CoreConfig,
    system_aad: &[u8],
) -> Result<Vec<Option<ResolvedHttpSecret>>, MappedErrors> {
    let mut decrypted_secrets: Vec<Option<ResolvedHttpSecret>> =
        Vec::with_capacity(hooks.len());

    for hook in hooks {
        let Some(secret) = hook.get_secret() else {
            decrypted_secrets.push(None);
            continue;
        };

        let decrypted = secret
            .decrypt_me(system_dek, &config.account_life_cycle, system_aad)
            .await
            .map_err(|err| {
                use_case_err(format!("Error on decrypting secret: {err}"))
            })?;

        let resolved = ResolvedHttpSecret::from_http_secret(decrypted)
            .await
            .map_err(|err| {
                use_case_err(format!("Error on resolving secret token: {err}"))
            })?;

        decrypted_secrets.push(Some(resolved));
    }

    Ok(decrypted_secrets)
}

/// Build the HTTP client used for every hook of this dispatch
async fn build_dispatch_client(
    config: &CoreConfig,
) -> Result<Client, MappedErrors> {
    let webhook = &config.webhook;

    let request_timeout =
        webhook.request_timeout_in_secs.async_get_or_error().await?;

    let connect_timeout =
        webhook.connect_timeout_in_secs.async_get_or_error().await?;

    Client::builder()
        .danger_accept_invalid_certs(
            webhook
                .accept_invalid_certificates
                .async_get_or_error()
                .await?,
        )
        .timeout(Duration::from_secs(request_timeout))
        .connect_timeout(Duration::from_secs(connect_timeout))
        .build()
        .map_err(|err| use_case_err(format!("Error on building client: {err}")))
}

/// Persist the outcome of one attempt
///
/// Every exit of `dispatch_webhooks` goes through here, which is what releases
/// the claim: the row leaves `Processing` and the dispatcher that picked it up
/// stops owning it. It is also the only place `attempts` is incremented, so an
/// exit that skipped it would leave the event retrying forever without ever
/// reaching the terminal state.
///
/// A `Failed` attempt that has spent the last allowed try is escalated to
/// `Exhausted` -- terminal, never selected again -- and announced, because
/// nothing else in the system would ever mention that the event was abandoned.
/// `Skipped` and `Success` are outcomes in their own right and are not
/// escalated.
///
async fn record_attempt(
    mut artifact: WebHookPayloadArtifact,
    status: WebHookExecutionStatus,
    max_attempts: u64,
    webhook_updating_repo: &dyn WebHookUpdating,
) -> Result<WebHookPayloadArtifact, MappedErrors> {
    // `attempts` is a `u8` on the artifact, so two things have to be said out
    // loud. The increment saturates: a plain `+ 1` at 255 panics in debug and
    // wraps to 0 in release, and a wrap puts the event back on tier 0 to retry
    // forever -- the exact silent loop this function exists to prevent. And the
    // configured ceiling is clamped into the same domain: `maxAttempts` is a
    // free `u64`, and anything above 255 could never be reached by a counter
    // that stops there, so the event would never be abandoned either.
    let ceiling = max_attempts.min(u8::MAX as u64) as u8;
    let attempts = artifact.attempts.unwrap_or(0).saturating_add(1);
    let exhausted =
        status == WebHookExecutionStatus::Failed && attempts >= ceiling;

    artifact.attempts = Some(attempts);
    artifact.status = Some(match exhausted {
        true => WebHookExecutionStatus::Exhausted,
        false => status,
    });

    if exhausted {
        tracing::error!(
            webhook_execution_id = %artifact.id.unwrap_or_default(),
            trigger = %artifact.trigger,
            attempts = attempts,
            "Webhook delivery abandoned after the last allowed attempt"
        );
    }

    match webhook_updating_repo
        .update_execution_event(artifact.to_owned())
        .await?
    {
        UpdatingResponseKind::NotUpdated(_, msg) => {
            tracing::error!("Error on updating webhook: {msg}");
            use_case_err("Error on updating webhook").as_error()
        }
        UpdatingResponseKind::Updated(artifact) => Ok(artifact),
    }
}

/// Record a failed attempt and hand back the error that caused it
///
/// The persistence is best-effort: if it fails too, the row is left claimed and
/// another pod reclaims it once the visibility window elapses, so the original
/// cause is the one worth propagating.
///
async fn abort_attempt(
    artifact: WebHookPayloadArtifact,
    max_attempts: u64,
    webhook_updating_repo: &dyn WebHookUpdating,
    message: String,
) -> MappedErrors {
    if let Err(err) = record_attempt(
        artifact,
        WebHookExecutionStatus::Failed,
        max_attempts,
        webhook_updating_repo,
    )
    .await
    {
        tracing::error!("Failed to record the aborted attempt: {err}");
    }

    tracing::error!("{message}");

    use_case_err(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::webhook::PayloadId;
    use async_trait::async_trait;
    use std::sync::Mutex;
    use uuid::Uuid;

    /// A `WebHookUpdating` that keeps whatever was written to it
    #[derive(Default)]
    struct CapturingRepository(Mutex<Option<WebHookPayloadArtifact>>);

    impl CapturingRepository {
        fn captured(&self) -> WebHookPayloadArtifact {
            self.0
                .lock()
                .unwrap()
                .to_owned()
                .expect("nothing was ever persisted")
        }
    }

    #[async_trait]
    impl WebHookUpdating for CapturingRepository {
        async fn update(
            &self,
            _: WebHook,
        ) -> Result<UpdatingResponseKind<WebHook>, MappedErrors> {
            unimplemented!("not exercised by these tests")
        }

        async fn update_execution_event(
            &self,
            artifact: WebHookPayloadArtifact,
        ) -> Result<UpdatingResponseKind<WebHookPayloadArtifact>, MappedErrors>
        {
            *self.0.lock().unwrap() = Some(artifact.to_owned());

            Ok(UpdatingResponseKind::Updated(artifact))
        }
    }

    fn artifact(attempts: Option<u8>) -> WebHookPayloadArtifact {
        let mut artifact = WebHookPayloadArtifact::new(
            Some(Uuid::new_v4()),
            "{}".to_string(),
            PayloadId::String("payload".to_string()),
            WebHookTrigger::SubscriptionAccountCreated,
        );

        artifact.attempts = attempts;
        artifact
    }

    #[tokio::test]
    async fn a_failure_short_of_the_ceiling_stays_retryable() {
        let repository = CapturingRepository::default();

        record_attempt(
            artifact(Some(3)),
            WebHookExecutionStatus::Failed,
            5,
            &repository,
        )
        .await
        .unwrap();

        let captured = repository.captured();

        assert_eq!(captured.attempts, Some(4));
        assert_eq!(captured.status, Some(WebHookExecutionStatus::Failed));
    }

    #[tokio::test]
    async fn the_last_allowed_failure_becomes_terminal() {
        let repository = CapturingRepository::default();

        record_attempt(
            artifact(Some(4)),
            WebHookExecutionStatus::Failed,
            5,
            &repository,
        )
        .await
        .unwrap();

        let captured = repository.captured();

        assert_eq!(captured.attempts, Some(5));
        assert_eq!(captured.status, Some(WebHookExecutionStatus::Exhausted));
    }

    #[tokio::test]
    async fn success_and_skipped_are_never_escalated() {
        for status in [
            WebHookExecutionStatus::Success,
            WebHookExecutionStatus::Skipped,
        ] {
            let repository = CapturingRepository::default();

            record_attempt(
                artifact(Some(9)),
                status.to_owned(),
                5,
                &repository,
            )
            .await
            .unwrap();

            assert_eq!(repository.captured().status, Some(status));
        }
    }

    #[tokio::test]
    async fn a_saturated_counter_still_terminates() {
        // `attempts` is a `u8`. A plain `+ 1` at 255 panics in debug and wraps
        // to 0 in release, and a wrap sends the event back to tier 0 to retry
        // forever. A ceiling above 255 is clamped for the same reason: an
        // unreachable ceiling is an event that is never abandoned.
        let repository = CapturingRepository::default();

        record_attempt(
            artifact(Some(u8::MAX)),
            WebHookExecutionStatus::Failed,
            u64::MAX,
            &repository,
        )
        .await
        .unwrap();

        let captured = repository.captured();

        assert_eq!(captured.attempts, Some(u8::MAX));
        assert_eq!(captured.status, Some(WebHookExecutionStatus::Exhausted));
    }

    #[tokio::test]
    async fn an_aborted_attempt_is_recorded_before_the_error_is_returned() {
        let repository = CapturingRepository::default();

        let error = abort_attempt(
            artifact(Some(4)),
            5,
            &repository,
            "the downstream secret could not be decrypted".to_string(),
        )
        .await;

        assert!(error
            .to_string()
            .contains("the downstream secret could not be decrypted"));

        // The point of D-7: an exit that never reached an HTTP request still
        // spends an attempt and still terminates, instead of leaving the event
        // claimed with its counter untouched.
        let captured = repository.captured();

        assert_eq!(captured.attempts, Some(5));
        assert_eq!(captured.status, Some(WebHookExecutionStatus::Exhausted));
    }
}
