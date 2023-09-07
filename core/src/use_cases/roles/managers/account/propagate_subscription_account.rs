use crate::{
    domain::{
        dtos::{account::Account, profile::Profile, webhook::HookTarget},
        entities::WebHookFetching,
    },
    use_cases::roles::managers::webhook::WebHookDefaultAction,
};

use clean_base::{
    entities::FetchManyResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use futures_util::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HookResponse {
    pub url: String,
    pub status: u16,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PropagationResponse {
    pub account: Account,
    pub propagation_responses: Option<Vec<HookResponse>>,
}

/// Propagate a new subscription account to all webhooks.
///
/// The propagation is done asynchronously, and the response is returned
/// immediately.
///
pub(super) async fn propagate_subscription_account(
    profile: Profile,
    account: Account,
    webhook_default_action: WebHookDefaultAction,
    hook_target: HookTarget,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<PropagationResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to register 
            subscription accounts."
                .to_string(),
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Propagate new account
    // ? -----------------------------------------------------------------------

    let target_hooks = match webhook_fetching_repo
        .list(Some(webhook_default_action.to_string()), Some(hook_target))
        .await?
    {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => Some(records),
        FetchManyResponseKind::FoundPaginated(paginated_records) => {
            Some(paginated_records.records)
        }
    };

    let client = Client::new();
    let mut propagation_responses: Option<Vec<HookResponse>> = None;

    if let Some(hooks) = target_hooks {
        //
        // Request bodies contains the account object as a JSON. It should be
        // parsed by upstream urls.
        //
        let bodies: Vec<_> = hooks
            .iter()
            .map(|hook| {
                let mut map = HashMap::new();
                map.insert("account", account.to_owned());
                client.post(hook.url.to_owned()).json(&map).send()
            })
            .collect();

        //
        // Propagation responses are collected and returned as a response. Users
        // can check if the propagation was successful.
        //
        let mut responses = Vec::<HookResponse>::new();
        for hook_res in join_all(bodies).await {
            let hook_res = hook_res.unwrap();

            responses.push(HookResponse {
                url: hook_res.url().to_string(),
                status: hook_res.status().as_u16(),
                body: hook_res.text().await.ok(),
            });
        }

        if !responses.is_empty() {
            propagation_responses = Some(responses);
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Return created account
    // ? -----------------------------------------------------------------------

    Ok(PropagationResponse {
        account,
        propagation_responses,
    })
}
