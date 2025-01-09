use super::connector::get_client;
use crate::prisma::{
    owner_on_tenant as owner_on_tenant_model, tenant as tenant_model,
    user as user_model,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        tenant::{TenantMeta, TenantMetaKey},
    },
    entities::TenantDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use prisma_client_rust::{
    operator::and, prisma_errors::query_engine::RecordNotFound, QueryError,
};
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, process::id as process_id};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TenantDeletion)]
pub struct TenantDeletionSqlDbRepository {}

#[async_trait]
impl TenantDeletion for TenantDeletionSqlDbRepository {
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
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
            .tenant()
            .delete(tenant_model::id::equals(id.to_owned().to_string()))
            .exec()
            .await
        {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }

    async fn delete_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Option<Uuid>,
        owner_email: Option<Email>,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
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

        let mut and_statement = vec![owner_on_tenant_model::tenant_id::equals(
            tenant_id.to_string(),
        )];

        if let Some(owner_id) = owner_id {
            and_statement.push(owner_on_tenant_model::owner_id::equals(
                owner_id.to_string(),
            ));
        }

        if let Some(owner_email) = owner_email {
            and_statement.push(owner_on_tenant_model::owner::is(vec![
                user_model::email::equals(owner_email.email()),
            ]));
        }

        match client
            ._transaction()
            .run(|client| async move {
                let owner = client
                    .owner_on_tenant()
                    .find_first(vec![and(and_statement)])
                    .exec()
                    .await?;

                if let Some(owner) = owner {
                    client
                        .owner_on_tenant()
                        .delete(owner_on_tenant_model::id::equals(owner.id))
                        .exec()
                        .await
                        .map(|_| ())
                } else {
                    Ok(())
                }
            })
            .await
        {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return creation_err(
                        "The specified owner is already registered on the tenant.",
                    )
                    .with_code(NativeErrorCodes::MYC00016)
                    .with_exp_true()
                    .as_error();
                };

                creation_err(format!("Could not create tenant: {err}"))
                    .as_error()
            }
        }
    }

    async fn delete_tenant_meta(
        &self,
        tenant_id: Uuid,
        key: TenantMetaKey,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
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

                let empty_map = TenantMeta::new();

                let mut updated_meta = if let Some(data) = tenant {
                    match data.meta.to_owned() {
                        Some(meta) => from_value(meta).unwrap(),
                        None => empty_map,
                    }
                } else {
                    empty_map
                };

                updated_meta.retain(|k, _| k != &key);

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
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }
}
