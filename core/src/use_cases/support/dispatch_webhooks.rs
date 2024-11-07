use crate::domain::{
    dtos::webhook::{
        HookResponse, WebHook, WebHookPropagationResponse, WebHookSecret,
        WebHookTrigger,
    },
    entities::WebHookFetching,
};

use futures_util::future::join_all;
use mycelium_base::entities::FetchManyResponseKind;
use reqwest::Client;
use tracing::error;

#[tracing::instrument(name = "dispatch_webhooks", skip_all)]
pub(crate) async fn dispatch_webhooks<
    PayloadBody: serde::ser::Serialize + Clone,
>(
    trigger: WebHookTrigger,
    payload_body: PayloadBody,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> WebHookPropagationResponse<PayloadBody> {
    // ? -----------------------------------------------------------------------
    // ? Initialize webhook response
    // ? -----------------------------------------------------------------------

    let mut webhook_response = WebHookPropagationResponse {
        payload: payload_body.to_owned(),
        propagations: None,
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
            error!("Error on fetching webhooks: {:?}", err);
            return webhook_response;
        }
    };

    let hooks: Vec<WebHook> = match hooks_fetching_response {
        FetchManyResponseKind::Found(records) => records,
        FetchManyResponseKind::NotFound => {
            return webhook_response;
        }
        _ => {
            error!("Webhook response should not be paginated");
            return webhook_response;
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Adjust the HTTP method given the trigger
    // ? -----------------------------------------------------------------------

    let method = match trigger {
        WebHookTrigger::CreateSubscriptionAccount
        | WebHookTrigger::CreateUserAccount
        | WebHookTrigger::InviteGuestAccount => "POST",
        WebHookTrigger::UpdateSubscriptionAccount
        | WebHookTrigger::UpdateUserAccount => "PUT",
        WebHookTrigger::DeleteSubscriptionAccount
        | WebHookTrigger::DeleteUserAccount
        | WebHookTrigger::UninviteGuestAccount => "DELETE",
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
        .map(|hook| {
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
                Some(secret) => match secret {
                    WebHookSecret::AuthorizationHeader {
                        name,
                        prefix,
                        token,
                    } => {
                        let credential_key = name
                            .to_owned()
                            .unwrap_or("Authorization".to_string());

                        let credential_value = if let Some(prefix) = prefix {
                            format!("{} {}", prefix, token)
                        } else {
                            token.to_owned()
                        };

                        base_request.header(credential_key, credential_value)
                    }
                    WebHookSecret::QueryParameter { name, token } => {
                        base_request
                            .query(&[(name.to_owned(), token.to_owned())])
                    }
                },
                None => base_request,
            })
            .json(&payload_body)
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
        let hook_res = match hook_res {
            Ok(res) => res,
            Err(err) => {
                responses.push(HookResponse {
                    url: "".to_string(),
                    status: 500,
                    body: Some(format!("Error on connect to webhook: {err}")),
                });

                continue;
            }
        };

        responses.push(HookResponse {
            url: hook_res.url().to_string(),
            status: hook_res.status().as_u16(),
            body: hook_res.text().await.ok(),
        });
    }

    // ? -----------------------------------------------------------------------
    // ? Update the response and return
    // ? -----------------------------------------------------------------------

    webhook_response.propagations = match responses.is_empty() {
        true => None,
        false => Some(responses),
    };

    webhook_response
}
