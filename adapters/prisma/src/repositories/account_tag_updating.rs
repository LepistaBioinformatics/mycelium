use super::connector::get_client;
use crate::prisma::account_tags as account_tags_model;

use async_trait::async_trait;
use myc_core::domain::{dtos::tag::Tag, entities::AccountTagUpdating};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use serde_json::{from_value, to_value};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountTagUpdating)]
pub struct AccountTagUpdatingSqlDbRepository {}

#[async_trait]
impl AccountTagUpdating for AccountTagUpdatingSqlDbRepository {
    // ? ----------------------------------------------------------------------
    // ? Abstract methods implementation
    // ? ----------------------------------------------------------------------

    async fn update(
        &self,
        tag: Tag,
    ) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .as_error()
            }
            Some(res) => res,
        };

        let response = client
            .account_tags()
            .update(
                account_tags_model::id::equals(tag.id.to_string()),
                vec![
                    account_tags_model::value::set(tag.value),
                    account_tags_model::meta::set(Some(
                        to_value(tag.meta).unwrap(),
                    )),
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => {
                return Ok(UpdatingResponseKind::Updated(Tag {
                    id: Uuid::parse_str(&record.id).unwrap(),
                    value: record.value,
                    meta: match record.meta {
                        None => None,
                        Some(meta) => Some(from_value(meta).unwrap()),
                    },
                }));
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        tag.id
                    ))
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {err}"
                ))
                .as_error();
            }
        }
    }
}
