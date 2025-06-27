use crate::{
    domain::{
        dtos::{
            http_secret::HttpSecret,
            webhook::{
                HookResponse, WebHook, WebHookExecutionStatus,
                WebHookPayloadArtifact, WebHookTrigger,
            },
        },
        entities::{WebHookFetching, WebHookUpdating},
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

#[tracing::instrument(
    name = "dispatch_webhooks",
    fields(trigger = %trigger, artifact_id = %artifact.id.unwrap_or_default()),
    skip(config, artifact, webhook_fetching_repo, webhook_updating_repo)
)]
pub async fn dispatch_webhooks(
    trigger: WebHookTrigger,
    artifact: WebHookPayloadArtifact,
    config: CoreConfig,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
    webhook_updating_repo: Box<&dyn WebHookUpdating>,
) -> Result<WebHookPayloadArtifact, MappedErrors> {
    let mut artifact = artifact.decode_payload()?;

    // ? -----------------------------------------------------------------------
    // ? Find for webhooks that are triggered by the event
    // ? -----------------------------------------------------------------------

    let hooks_fetching_response = match webhook_fetching_repo
        .list_by_trigger(trigger.to_owned())
        .await
    {
        Ok(response) => response,
        Err(err) => {
            return use_case_err(format!("Error on fetching webhooks: {err}"))
                .as_error();
        }
    };

    let hooks: Vec<WebHook> = match hooks_fetching_response {
        FetchManyResponseKind::Found(records) => records,
        FetchManyResponseKind::NotFound => {
            //
            // Update the artifact with the status of the event
            //
            artifact.status = Some(WebHookExecutionStatus::Skipped);
            artifact.attempts = Some(artifact.attempts.unwrap_or(0) + 1);

            webhook_updating_repo
                .update_execution_event(artifact.encode_payload()?)
                .await?;

            return Ok(artifact);
        }
        _ => {
            return use_case_err("Webhook response should not be paginated")
                .as_error();
        }
    };

    tracing::info!("Found {} webhooks to dispatch", hooks.len());

    // ? -----------------------------------------------------------------------
    // ? Adjust the HTTP method given the trigger
    // ? -----------------------------------------------------------------------

    let method = match trigger {
        WebHookTrigger::SubscriptionAccountCreated
        | WebHookTrigger::UserAccountCreated => "POST",
        WebHookTrigger::SubscriptionAccountUpdated
        | WebHookTrigger::UserAccountUpdated => "PUT",
        WebHookTrigger::SubscriptionAccountDeleted
        | WebHookTrigger::UserAccountDeleted => "DELETE",
    };

    // ? -----------------------------------------------------------------------
    // ? Build requests to the webhooks
    //
    // Request bodies contains the account object as a JSON. It should be parsed
    // by upstream urls.
    //
    // ? -----------------------------------------------------------------------

    let client = Client::builder()
        .danger_accept_invalid_certs(
            config
                .webhook
                .accept_invalid_certificates
                .async_get_or_error()
                .await?,
        )
        .build()
        .map_err(|err| {
            use_case_err(format!("Error on building client: {err}"))
        })?;

    let bodies: Vec<_> = hooks
        .iter()
        .map(|hook| async {
            //
            // Create a base request to the webhook url
            //
            let base_request = client.clone();
            //
            // Build the request based on the method
            //
            let base_request = match method {
                "POST" => base_request.post(hook.url.to_owned()),
                "PUT" => base_request.put(hook.url.to_owned()),
                "DELETE" => base_request.delete(match artifact.id {
                    None => hook.url.to_owned(),
                    Some(id) => format!("{}/{}", hook.url, id),
                }),
                _ => {
                    tracing::error!("Unknown method: {method}");
                    base_request.post(hook.url.to_owned())
                }
            };
            //
            // Attach the secret to the request if it exists
            //
            (match &hook.get_secret() {
                Some(secret) => {
                    let decrypted_secret = match secret
                        .decrypt_me(config.account_life_cycle.to_owned())
                        .await
                    {
                        Ok(secret) => secret,
                        Err(err) => {
                            panic!("Error on decrypting secret: {:?}", err);
                        }
                    };

                    match decrypted_secret {
                        HttpSecret::AuthorizationHeader {
                            header_name,
                            prefix,
                            token,
                        } => {
                            let credential_key = header_name
                                .to_owned()
                                .unwrap_or("Authorization".to_string());

                            let credential_value = if let Some(prefix) = prefix
                            {
                                format!("{} {}", prefix, token)
                            } else {
                                token.to_owned()
                            };

                            base_request
                                .header(credential_key, credential_value)
                        }
                        HttpSecret::QueryParameter { name, token } => {
                            base_request
                                .query(&[(name.to_owned(), token.to_owned())])
                        }
                    }
                }
                None => base_request,
            })
            .body(artifact.payload.to_owned())
            .header("Content-Type", "application/json")
            .send()
        })
        .collect();

    tracing::info!("Sending {} webhooks", bodies.len());

    // ? -----------------------------------------------------------------------
    // ? Propagate responses
    //
    // Propagation responses are collected and returned as a response. Users can
    // check if the propagation was successful.
    //
    // ? -----------------------------------------------------------------------

    let mut responses = Vec::<HookResponse>::new();
    for hook_future in join_all(bodies).await {
        let hook_res = match hook_future.await {
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

    let status = if responses.iter().any(|response| response.status >= 400) {
        WebHookExecutionStatus::Failed
    } else {
        WebHookExecutionStatus::Success
    };

    // ? -----------------------------------------------------------------------
    // ? Update artifact with propagation responses
    // ? -----------------------------------------------------------------------

    artifact.attempts = Some(artifact.attempts.unwrap_or(0) + 1);
    artifact.status = Some(status);

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

    match webhook_updating_repo
        .update_execution_event(artifact.to_owned())
        .await?
    {
        UpdatingResponseKind::NotUpdated(_, msg) => {
            tracing::error!("Error on updating webhook: {msg}");

            return use_case_err("Error on updating webhook").as_error();
        }
        UpdatingResponseKind::Updated(artifact) => Ok(artifact),
    }
}
