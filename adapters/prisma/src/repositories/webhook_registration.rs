use crate::{
    prisma::webhook as webhook_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, webhook::WebHook},
    entities::WebHookRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookRegistration)]
pub struct WebHookRegistrationSqlDbRepository {}

#[async_trait]
impl WebHookRegistration for WebHookRegistrationSqlDbRepository {
    async fn create(
        &self,
        webhook: WebHook,
    ) -> Result<CreateResponseKind<WebHook>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build create part of the get-or-create
        // ? -------------------------------------------------------------------

        match client
            .webhook()
            .create(
                webhook.name.to_owned(),
                webhook.target.to_owned().to_string(),
                webhook.url.to_owned(),
                vec![webhook_model::description::set(
                    webhook.description.to_owned(),
                )],
            )
            .exec()
            .await
        {
            Ok(record) => Ok(CreateResponseKind::Created(WebHook {
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
            })),
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {err}"
                ))
                .as_error();
            }
        }
    }
}
