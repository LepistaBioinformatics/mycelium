use crate::domain::dtos::{
    account::Account,
    webhook::{HookResponse, WebHook},
};

use futures_util::future::join_all;
use reqwest::Client;
use std::collections::HashMap;

pub(crate) async fn dispatch_webhooks(
    hooks: Vec<WebHook>,
    account: Account,
    bearer_token: Option<String>,
) -> Option<Vec<HookResponse>> {
    let client = Client::new();

    // ? -----------------------------------------------------------------------
    // ? Request bodies contains the account object as a JSON. It should be
    // ? parsed by upstream urls.
    // ? -----------------------------------------------------------------------

    let bodies: Vec<_> = hooks
        .iter()
        .map(|hook| {
            let mut map = HashMap::new();
            map.insert("account", account.to_owned());
            client
                .clone()
                .post(hook.url.to_owned())
                .header(
                    "Authorization",
                    bearer_token.to_owned().unwrap_or("".to_string()),
                )
                .json(&map)
                .send()
        })
        .collect();

    // ? -----------------------------------------------------------------------
    // ? Propagation responses are collected and returned as a response. Users
    // ? can check if the propagation was successful.
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

    match responses.is_empty() {
        true => None,
        false => Some(responses),
    }
}
