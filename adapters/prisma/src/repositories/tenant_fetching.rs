use super::connector::get_client;
use crate::prisma::{
    manager_account_on_tenant as manager_account_on_tenant_model,
    owner_on_tenant as owner_on_tenant_model, tenant as tenant_model,
    tenant_tag as tenant_tag_model, QueryMode,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::Owner,
        tenant::{Tenant, TenantMetaKey},
    },
    entities::TenantFetching,
};
use mycelium_base::{
    dtos::Children,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use prisma_client_rust::{and, operator::and as and_o};
use serde_json::to_value;
use shaku::Component;
use std::{collections::HashMap, process::id as process_id};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantFetching)]
pub struct TenantFetchingSqlDbRepository {}

#[async_trait]
impl TenantFetching for TenantFetchingSqlDbRepository {
    async fn get_tenant_owned_by_me(
        &self,
        id: Uuid,
        owners_ids: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        let and_statement =
            vec![tenant_model::owners::some(vec![and_o(vec![
                owner_on_tenant_model::owner_id::in_vec(
                    owners_ids.iter().map(|id| id.to_string()).collect(),
                ),
                owner_on_tenant_model::tenant_id::equals(id.to_string()),
            ])])];

        match client
            .tenant()
            .find_first(vec![and_o(and_statement)])
            .include(tenant_model::include!({
                owners: select {
                    id
                    owner: select {
                        id
                        email
                        first_name
                        last_name
                        username
                    }
                }
            }))
            .exec()
            .await
        {
            Ok(record) => {
                if let Some(record) = record {
                    Ok(FetchResponseKind::Found(Tenant {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description,
                        owners: Children::Records(
                            record
                                .owners
                                .iter()
                                .map(|owner| {
                                    let owner = owner.owner.to_owned();

                                    Owner {
                                        id: Uuid::parse_str(&owner.id).unwrap(),
                                        email: owner.email.clone(),
                                        first_name: owner
                                            .first_name
                                            .clone()
                                            .into(),
                                        last_name: owner
                                            .last_name
                                            .clone()
                                            .into(),
                                        username: owner.username.clone().into(),
                                    }
                                })
                                .collect(),
                        ),
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(updated) => Some(updated.into()),
                        },
                        manager: None,
                        tags: None,
                        meta: None,
                        status: None,
                    }))
                } else {
                    Ok(FetchResponseKind::NotFound(Some(id.to_string())))
                }
            }
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }

    async fn get_for_tenants_by_manager_account(
        &self,
        id: Uuid,
        manager_ids: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        let and_statement = vec![tenant_model::manager::is(vec![
            manager_account_on_tenant_model::account_id::in_vec(
                manager_ids.iter().map(|id| id.to_string()).collect(),
            ),
            manager_account_on_tenant_model::tenant_id::equals(id.to_string()),
        ])];

        match client
            .tenant()
            .find_first(vec![and_o(and_statement)])
            .include(tenant_model::include!({
                owners: select {
                    id
                    owner: select {
                        id
                        email
                        first_name
                        last_name
                        username
                    }
                }
            }))
            .exec()
            .await
        {
            Ok(record) => {
                if let Some(record) = record {
                    Ok(FetchResponseKind::Found(Tenant {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description,
                        owners: Children::Records(
                            record
                                .owners
                                .iter()
                                .map(|owner| {
                                    let owner = owner.owner.to_owned();

                                    Owner {
                                        id: Uuid::parse_str(&owner.id).unwrap(),
                                        email: owner.email.clone(),
                                        first_name: owner
                                            .first_name
                                            .clone()
                                            .into(),
                                        last_name: owner
                                            .last_name
                                            .clone()
                                            .into(),
                                        username: owner.username.clone().into(),
                                    }
                                })
                                .collect(),
                        ),
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(updated) => Some(updated.into()),
                        },
                        manager: None,
                        tags: None,
                        meta: None,
                        status: None,
                    }))
                } else {
                    Ok(FetchResponseKind::NotFound(Some(id.to_string())))
                }
            }
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }

    async fn filter_tenants_as_manager(
        &self,
        name: Option<String>,
        owner: Option<Uuid>,
        metadata_key: Option<TenantMetaKey>,
        status_verified: Option<bool>,
        status_archived: Option<bool>,
        status_trashed: Option<bool>,
        tag_value: Option<String>,
        tag_meta: Option<String>,
    ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        let mut and_statement = vec![];

        if let Some(name) = name {
            and_statement.push(and![
                tenant_model::name::mode(QueryMode::Insensitive),
                tenant_model::name::contains(name),
            ]);
        }

        if let Some(owner) = owner {
            and_statement.push(tenant_model::owners::some(vec![
                owner_on_tenant_model::owner_id::equals(owner.to_string()),
            ]));
        }

        if let Some(metadata_key) = metadata_key {
            and_statement.push(tenant_model::meta::string_contains(
                metadata_key.to_string(),
            ));
        }

        if let Some(status_verified) = status_verified {
            let mut map = HashMap::new();
            map.insert("verified".to_string(), status_verified.to_string());

            and_statement.push(tenant_model::status::has(Some(
                to_value(serde_json::to_string(&map).unwrap()).unwrap(),
            )));
        }

        if let Some(status_archived) = status_archived {
            let mut map = HashMap::new();
            map.insert("archived".to_string(), status_archived.to_string());

            and_statement.push(tenant_model::status::has(Some(
                to_value(serde_json::to_string(&map).unwrap()).unwrap(),
            )));
        }

        if let Some(status_trashed) = status_trashed {
            let mut map = HashMap::new();
            map.insert("trashed".to_string(), status_trashed.to_string());

            and_statement.push(tenant_model::status::has(Some(
                to_value(serde_json::to_string(&map).unwrap()).unwrap(),
            )));
        }

        if let Some(tag_value) = tag_value {
            and_statement.push(tenant_model::tags::some(vec![and![
                tenant_tag_model::value::mode(QueryMode::Insensitive),
                tenant_tag_model::value::contains(tag_value),
            ]]));
        }

        if let Some(tag_meta) = tag_meta {
            and_statement.push(tenant_model::tags::some(vec![
                tenant_tag_model::meta::string_contains(tag_meta),
            ]));
        }

        match client
            .tenant()
            .find_many(vec![and_o(and_statement)])
            .include(tenant_model::include!({
                owners: select {
                    id
                    owner: select {
                        id
                        email
                        first_name
                        last_name
                        username
                    }
                }
            }))
            .exec()
            .await
        {
            Ok(records) => {
                let parsed_records: Vec<Tenant> = records
                    .iter()
                    .map(|record| Tenant {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name.to_owned(),
                        description: record.description.to_owned(),
                        owners: Children::Records(
                            record
                                .owners
                                .iter()
                                .map(|owner| {
                                    let owner = owner.owner.to_owned();

                                    Owner {
                                        id: Uuid::parse_str(&owner.id).unwrap(),
                                        email: owner.email.clone(),
                                        first_name: owner
                                            .first_name
                                            .clone()
                                            .into(),
                                        last_name: owner
                                            .last_name
                                            .clone()
                                            .into(),
                                        username: owner.username.clone().into(),
                                    }
                                })
                                .collect(),
                        ),
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(updated) => Some(updated.into()),
                        },
                        manager: None,
                        tags: None,
                        meta: None,
                        status: None,
                    })
                    .collect();

                if parsed_records.is_empty() {
                    Ok(FetchManyResponseKind::NotFound)
                } else {
                    Ok(FetchManyResponseKind::Found(parsed_records))
                }
            }
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }
}
