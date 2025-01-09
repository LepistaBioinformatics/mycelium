use super::connector::get_client;
use crate::prisma::{tenant as tenant_model, tenant_tag as tenant_tag_model};

use async_trait::async_trait;
use myc_core::domain::{dtos::tag::Tag, entities::TenantTagRegistration};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, process::id as process_id};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantTagRegistration)]
pub struct TenantTagRegistrationSqlDbRepository {}

#[async_trait]
impl TenantTagRegistration for TenantTagRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        tenant_id: Uuid,
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
            .tenant_tag()
            .find_first(vec![
                tenant_tag_model::value::equals(tag.to_owned()),
                tenant_tag_model::tenant_id::equals(
                    tenant_id.to_string().to_owned(),
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
            .tenant_tag()
            .create(
                tag.to_owned(),
                tenant_model::id::equals(tenant_id.to_string().to_owned()),
                vec![tenant_tag_model::meta::set(Some(
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
