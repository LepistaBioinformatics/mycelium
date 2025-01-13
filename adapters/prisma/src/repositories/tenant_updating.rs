use super::connector::get_client;
use crate::prisma::{tenant as tenant_model, user as user_model};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::Owner,
        tenant::{Tenant, TenantMeta, TenantMetaKey, TenantStatus},
    },
    entities::{TenantOwnerConnection, TenantUpdating},
};
use mycelium_base::{
    dtos::Children,
    entities::{CreateResponseKind, UpdatingResponseKind},
    utils::errors::{updating_err, MappedErrors},
};
use prisma_client_rust::{
    prisma_errors::query_engine::UniqueKeyViolation, QueryError,
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, process::id as process_id};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantUpdating)]
pub struct TenantUpdatingSqlDbRepository {}

#[async_trait]
impl TenantUpdating for TenantUpdatingSqlDbRepository {
    async fn update_name_and_description(
        &self,
        tenant_id: Uuid,
        tenant: Tenant,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        match client
            .tenant()
            .update(
                tenant_model::id::equals(tenant_id.to_string()),
                vec![
                    tenant_model::name::set(tenant.name),
                    tenant_model::description::set(tenant.description),
                ],
            )
            .include(tenant_model::include!({
                owners: select {
                    id
                    owner: select {
                        id
                        email
                        first_name
                        last_name
                        username
                        is_principal
                    }
                }
            }))
            .exec()
            .await
        {
            Ok(record) => Ok(UpdatingResponseKind::Updated(Tenant {
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
                                first_name: owner.first_name.clone().into(),
                                last_name: owner.last_name.clone().into(),
                                username: owner.username.clone().into(),
                                is_principal: owner.is_principal,
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
            })),
            Err(err) => updating_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }

    async fn update_tenant_status(
        &self,
        tenant_id: Uuid,
        status: TenantStatus,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
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
                    .select(tenant_model::select!({ status }))
                    .exec()
                    .await?;

                let new_status = if let Some(data) = tenant {
                    let mut statuses: Vec<TenantStatus> = data
                        .status
                        .iter()
                        .map(|status| from_value(status.to_owned()).unwrap())
                        .collect();

                    statuses.push(status);
                    statuses
                } else {
                    vec![status]
                };

                client
                    .tenant()
                    .update(
                        tenant_model::id::equals(tenant_id.to_string()),
                        vec![tenant_model::status::set(
                            new_status
                                .iter()
                                .map(|status| to_value(status).unwrap())
                                .collect(),
                        )],
                    )
                    .include(tenant_model::include!({
                        owners: select {
                            id
                            owner: select {
                                id
                                email
                                first_name
                                last_name
                                username
                                is_principal
                            }
                        }
                    }))
                    .exec()
                    .await
            })
            .await
        {
            Ok(record) => Ok(UpdatingResponseKind::Updated(Tenant {
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
                                first_name: owner.first_name.clone().into(),
                                last_name: owner.last_name.clone().into(),
                                username: owner.username.clone().into(),
                                is_principal: owner.is_principal,
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
                status: record
                    .status
                    .iter()
                    .map(|status| from_value(status.to_owned()).unwrap())
                    .collect(),
            })),
            Err(err) => updating_err(format!("Could not create tenant: {err}"))
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
                return updating_err(String::from(
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
            Err(err) => {
                if err.is_prisma_error::<UniqueKeyViolation>() {
                    return updating_err(
                        "The specified owner is already registered on the tenant.",
                    )
                    .with_code(NativeErrorCodes::MYC00015)
                    .with_exp_true()
                    .as_error();
                };

                updating_err(format!("Could not create tenant owner: {err}"))
                    .as_error()
            }
        }
    }

    async fn update_tenant_meta(
        &self,
        tenant_id: Uuid,
        key: TenantMetaKey,
        value: String,
    ) -> Result<UpdatingResponseKind<TenantMeta>, MappedErrors> {
        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
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

                let empty_map = TenantMeta::new();
                let mut updated_meta: TenantMeta = if let Some(data) = tenant {
                    match data.meta.to_owned() {
                        Some(meta) => from_value(meta).unwrap(),
                        None => empty_map,
                    }
                } else {
                    empty_map
                };

                updated_meta.insert(key, value);

                client
                    .tenant()
                    .update(
                        tenant_model::id::equals(tenant_id.to_string()),
                        vec![tenant_model::meta::set(Some(
                            to_value(updated_meta.to_owned()).unwrap(),
                        ))],
                    )
                    .exec()
                    .await?;

                Ok::<HashMap<TenantMetaKey, String>, QueryError>(updated_meta)
            })
            .await
        {
            Ok(record) => Ok(UpdatingResponseKind::Updated(record)),
            Err(err) => updating_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }
}
