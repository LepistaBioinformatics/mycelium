use super::connector::get_client;
use crate::prisma::account as account_model;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        account::{AccountMeta, AccountMetaKey},
        native_error_codes::NativeErrorCodes,
        related_accounts::RelatedAccounts,
    },
    entities::AccountDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use prisma_client_rust::QueryError;
use serde_json::{from_value, to_value};
use shaku::Component;
use std::{collections::HashMap, process::id as process_id};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountDeletion)]
pub struct AccountDeletionSqlDbRepository {}

#[async_trait]
impl AccountDeletion for AccountDeletionSqlDbRepository {
    async fn delete(
        &self,
        account_id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
            if !ids.contains(&account_id) {
                return creation_err(String::from(
                    "Account deletion error. Account not allowed to be deleted.",
                ))
                .as_error();
            }
        }

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
            .account()
            .delete(account_model::id::equals(
                account_id.to_owned().to_string(),
            ))
            .exec()
            .await
        {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }

    async fn delete_account_meta(
        &self,
        account_id: Uuid,
        key: AccountMetaKey,
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
                    .account()
                    .find_unique(account_model::id::equals(
                        account_id.to_string(),
                    ))
                    .select(account_model::select!({ meta }))
                    .exec()
                    .await?;

                let empty_map = AccountMeta::new();

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
                    .account()
                    .update(
                        account_model::id::equals(account_id.to_string()),
                        vec![account_model::meta::set(Some(
                            to_value(updated_meta.to_owned()).unwrap(),
                        ))],
                    )
                    .exec()
                    .await?;

                Ok::<HashMap<AccountMetaKey, String>, QueryError>(updated_meta)
            })
            .await
        {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(err) => creation_err(format!("Could not create tenant: {err}"))
                .as_error(),
        }
    }
}
