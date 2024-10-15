use super::connector::get_client;
use crate::prisma::{
    account as account_model, account_tags as result_tags_model,
};

use async_trait::async_trait;
use myc_core::domain::{dtos::tag::Tag, entities::AccountTagRegistration};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, process::id as process_id};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountTagRegistration)]
pub struct AccountTagRegistrationSqlDbRepository {}

#[async_trait]
impl AccountTagRegistration for AccountTagRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        analysis_id: Uuid,
        tag: String,
        meta: HashMap<String, String>,
    ) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .as_error()
            }
            Some(res) => res,
        };

        let response = client
            .account_tags()
            .find_first(vec![
                result_tags_model::value::equals(tag.to_owned()),
                result_tags_model::account_id::equals(
                    analysis_id.to_string().to_owned(),
                ),
            ])
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                return Ok(GetOrCreateResponseKind::NotCreated(
                    Tag {
                        id: Uuid::parse_str(&record.id).unwrap(),
                        value: record.value,
                        meta: match record.meta {
                            None => None,
                            Some(meta) => Some(from_value(meta).unwrap()),
                        },
                    },
                    "Tag already exists".to_string(),
                ));
            }
            None => (),
        };

        let response = client
            .account_tags()
            .create(
                tag.to_owned(),
                account_model::id::equals(analysis_id.to_string().to_owned()),
                vec![result_tags_model::meta::set(Some(
                    to_value(meta).unwrap(),
                ))],
            )
            .exec()
            .await;

        match response {
            Ok(record) => Ok(GetOrCreateResponseKind::Created(Tag {
                id: Uuid::parse_str(&record.id).unwrap(),
                value: record.value,
                meta: match record.meta {
                    None => None,
                    Some(meta) => Some(from_value(meta).unwrap()),
                },
            })),
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on update record: {err}"
                ))
                .as_error();
            }
        }
    }
}
