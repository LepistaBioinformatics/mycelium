use super::connector::get_client;
use crate::prisma::{
    owner_on_tenant as owner_on_tenant_model, tenant as tenant_model,
    user as user_model,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::Owner,
        tenant::{Tenant, TenantMeta, TenantMetaKey},
    },
    entities::{TenantOwnerConnection, TenantRegistration},
};
use mycelium_base::{
    dtos::Children,
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantRegistration)]
pub struct TenantRegistrationSqlDbRepository {}

#[async_trait]
impl TenantRegistration for TenantRegistrationSqlDbRepository {
    async fn create(
        &self,
        tenant: Tenant,
        guest_by: String,
    ) -> Result<CreateResponseKind<Tenant>, MappedErrors> {
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

        match client
            ._transaction()
            .run(|client| async move {
                let new_tenant = client
                    .tenant()
                    .create(
                        tenant.name,
                        vec![tenant_model::description::set(
                            tenant.description,
                        )],
                    )
                    .exec()
                    .await?;

                let owners_ids = match tenant.owners {
                    Children::Records(owners) => {
                        owners.iter().map(|owner| owner.id).collect()
                    }
                    Children::Ids(ids) => ids,
                };

                client
                    .owner_on_tenant()
                    .create_many(
                        owners_ids
                            .iter()
                            .map(|id| {
                                owner_on_tenant_model::create_unchecked(
                                    new_tenant.id.to_owned(),
                                    id.to_string(),
                                    guest_by.to_owned(),
                                    vec![],
                                )
                            })
                            .collect(),
                    )
                    .exec()
                    .await?;

                client
                    .tenant()
                    .find_unique(tenant_model::id::equals(new_tenant.id))
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
            })
            .await
        {
            Ok(record) => {
                if let Some(record) = record {
                    Ok(CreateResponseKind::Created(Tenant {
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
                    creation_err(String::from("Could not create tenant"))
                        .as_error()
                }
            }
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }

    async fn register_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Uuid,
        guest_by: String,
    ) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors> {
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

        match client
            .owner_on_tenant()
            .create(
                tenant_model::id::equals(tenant_id.to_owned().to_string()),
                user_model::id::equals(owner_id.to_string()),
                guest_by,
                vec![],
            )
            .exec()
            .await
        {
            Ok(record) => {
                Ok(CreateResponseKind::Created(TenantOwnerConnection {
                    tenant_id: Uuid::parse_str(&record.tenant_id).unwrap(),
                    owner_id: Uuid::parse_str(&record.owner_id).unwrap(),
                    guest_by: record.guest_by,
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(updated) => Some(updated.into()),
                    },
                }))
            }
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }

    async fn register_tenant_meta(
        &self,
        tenant_id: Uuid,
        key: TenantMetaKey,
        value: String,
    ) -> Result<CreateResponseKind<TenantMeta>, MappedErrors> {
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

        match client
            ._transaction()
            .run(|client| async move {
                let tenant = client
                    .tenant()
                    .find_unique(tenant_model::id::equals(
                        tenant_id.to_string(),
                    ))
                    .select(tenant_model::select!({ meta }))
                    .exec()
                    .await?;

                let new_meta = if let Some(data) = tenant {
                    match data.meta {
                        Some(meta) => {
                            let mut map: HashMap<String, String> =
                                from_value(meta).unwrap();

                            map.insert(key.to_string(), value);
                            map
                        }
                        None => {
                            let mut map: HashMap<String, String> =
                                std::collections::HashMap::new();

                            map.insert(key.to_string(), value);
                            map
                        }
                    }
                } else {
                    let mut map: HashMap<String, String> =
                        std::collections::HashMap::new();

                    map.insert(key.to_string(), value);
                    map
                };

                client
                    .tenant()
                    .update(
                        tenant_model::id::equals(tenant_id.to_string()),
                        vec![tenant_model::meta::set(Some(
                            to_value(&new_meta).unwrap(),
                        ))],
                    )
                    .select(tenant_model::select!({ meta }))
                    .exec()
                    .await
            })
            .await
        {
            Ok(record) => {
                if let Some(meta) = record.meta {
                    Ok(CreateResponseKind::Created(TenantMeta::from_iter(
                        meta.as_object().unwrap().iter().map(|(k, v)| {
                            (TenantMetaKey::from_str(k).unwrap(), v.to_string())
                        }),
                    )))
                } else {
                    creation_err(String::from("Could not create tenant meta"))
                        .as_error()
                }
            }
            Err(err) => {
                creation_err(format!("Could not create tenant meta: {err}"))
                    .as_error()
            }
        }
    }
}
