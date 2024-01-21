use crate::{
    prisma::webhook as webhook_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use clean_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{factories::fetching_err, MappedErrors},
};
use log::debug;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{HookTarget, WebHook},
    },
    entities::WebHookFetching,
};
use prisma_client_rust::operator::and;
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookFetching)]
pub struct WebHookFetchingSqlDbRepository {}

#[async_trait]
impl WebHookFetching for WebHookFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .webhook()
            .find_unique(webhook_model::id::equals(id.to_owned().to_string()))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(id))),
                Some(record) => Ok(FetchResponseKind::Found(WebHook {
                    id: Some(Uuid::from_str(&record.id).unwrap()),
                    name: record.name,
                    description: record.description.into(),
                    target: record.target.parse().unwrap(),
                    url: record.url,
                    is_active: record.is_active,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(date) => Some(date.with_timezone(&Local)),
                    },
                })),
            },
        }
    }

    async fn list(
        &self,
        name: Option<String>,
        target: Option<HookTarget>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build list query statement
        // ? -------------------------------------------------------------------

        let mut and_stmt = vec![];
        let mut query_stmt = vec![];

        if name.is_some() {
            and_stmt.push(webhook_model::name::contains(name.unwrap()))
        }

        if target.is_some() {
            and_stmt.push(webhook_model::target::contains(
                target.unwrap().to_string(),
            ))
        }

        if !and_stmt.is_empty() {
            query_stmt.push(and(and_stmt))
        }

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client.webhook().find_many(query_stmt).exec().await {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on fetch webhooks: {err}",
                ))
                .as_error()
            }
            Ok(res) => {
                let response = res
                    .into_iter()
                    .map(|record| WebHook {
                        id: Some(Uuid::from_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description,
                        target: record.target.parse().unwrap(),
                        url: record.url,
                        is_active: record.is_active,
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                    })
                    .collect::<Vec<WebHook>>();

                debug!("Webhooks found: {:?}", response);

                if response.len() == 0 {
                    return Ok(FetchManyResponseKind::NotFound);
                }

                Ok(FetchManyResponseKind::Found(response))
            }
        }
    }
}
