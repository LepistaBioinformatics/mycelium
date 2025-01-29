use crate::{
    domain::{
        dtos::{
            http_secret::HttpSecret,
            webhook::{
                HookResponse, WebHook, WebHookPropagationArtifact,
                WebHookTrigger,
            },
        },
        entities::WebHookFetching,
    },
    models::AccountLifeCycle,
};

use futures_util::future::join_all;
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use reqwest::Client;
use tracing::error;

#[tracing::instrument(name = "dispatch_webhooks", skip_all)]
pub async fn dispatch_webhooks<PayloadBody: serde::ser::Serialize + Clone>(
    trigger: WebHookTrigger,
    mut artifact: WebHookPropagationArtifact,
    config: AccountLifeCycle,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<WebHookPropagationArtifact, MappedErrors> {
    artifact = artifact.encode_payload()?;

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
            return Ok(artifact);
        }
        _ => {
            return use_case_err("Webhook response should not be paginated")
                .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Adjust the HTTP method given the trigger
    // ? -----------------------------------------------------------------------

    let method = match trigger {
        WebHookTrigger::SubscriptionAccountCreated
        | WebHookTrigger::UserAccountCreated
        | WebHookTrigger::GuestAccountInvited => "POST",
        WebHookTrigger::SubscriptionAccountUpdated
        | WebHookTrigger::UserAccountUpdated => "PUT",
        WebHookTrigger::SubscriptionAccountDeleted
        | WebHookTrigger::UserAccountDeleted
        | WebHookTrigger::GuestAccountRevoked => "DELETE",
    };

    // ? -----------------------------------------------------------------------
    // ? Build requests to the webhooks
    //
    // Request bodies contains the account object as a JSON. It should be parsed
    // by upstream urls.
    //
    // ? -----------------------------------------------------------------------

    let client = Client::new();

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
                "DELETE" => base_request.delete(hook.url.to_owned()),
                _ => {
                    error!("Unknown method: {method}");
                    base_request.post(hook.url.to_owned())
                }
            };
            //
            // Attach the secret to the request if it exists
            //
            (match &hook.get_secret() {
                Some(secret) => {
                    let decrypted_secret =
                        match secret.decrypt_me(config.to_owned()).await {
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
            .json(&artifact)
            .send()
        })
        .collect();

    // ? -----------------------------------------------------------------------
    // ? Propagate responses
    //
    // Propagation responses are collected and returned as a response. Users can
    // check if the propagation was successful.
    //
    // ? -----------------------------------------------------------------------

    let mut responses = Vec::<HookResponse>::new();
    for hook_res in join_all(bodies).await {
        let hook_res = match hook_res.await {
            Ok(res) => res,
            Err(err) => {
                error!("Error on connect to webhook: {:?}", err);

                responses.push(HookResponse {
                    url: "".to_string(),
                    status: 500,
                    body: Some("Error on connect to webhook".to_string()),
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
        });
    }

    // ? -----------------------------------------------------------------------
    // ? Update the response and return
    // ? -----------------------------------------------------------------------

    artifact.propagations = match responses.is_empty() {
        true => None,
        false => Some(responses),
    };

    Ok(artifact)
}
