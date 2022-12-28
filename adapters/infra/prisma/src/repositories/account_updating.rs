use crate::{
    prisma::account as account_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use myc_core::domain::{dtos::account::AccountDTO, entities::AccountUpdating};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountUpdating)]
pub struct AccountUpdatingSqlDbRepository {}

#[async_trait]
impl AccountUpdating for AccountUpdatingSqlDbRepository {
    async fn update(
        &self,
        account: AccountDTO,
    ) -> Result<UpdatingResponseKind<AccountDTO>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(updating_err(
                    String::from(
                        "Prisma Client error. Could not fetch client.",
                    ),
                    Some(false),
                    None,
                ))
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to update record
        // ? -------------------------------------------------------------------

        let account_id = match account.id {
            None => {
                return Err(updating_err(
                    String::from("Unable to update account. Invalid record ID"),
                    None,
                    None,
                ))
            }
            Some(res) => res,
        };

        let response = client
            .account()
            .update(
                account_model::id::equals(account_id.to_string()),
                vec![
                    account_model::name::set(account.name),
                    account_model::is_active::set(account.is_active),
                    account_model::is_checked::set(account.is_checked),
                    account_model::account_type_id::set(match account.account_type {
                        ParentEnum::Id(id) => id.to_string(),
                        ParentEnum::Record(record) => match record.id {
                            None => {
                                return Err(updating_err(
                                    String::from("Unable to update account. Invalid account type ID"),
                                    None,
                                    None,
                                ))
                            }
                            Some(id) => id.to_string(),
                        }
                    })
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => Ok(UpdatingResponseKind::Updated(AccountDTO {
                id: Some(Uuid::parse_str(&record.id).unwrap()),
                name: record.name,
                is_active: record.is_active,
                is_checked: record.is_checked,
                owner: ParentEnum::Id(
                    Uuid::from_str(&record.owner_id).unwrap(),
                ),
                account_type: ParentEnum::Id(
                    Uuid::from_str(&record.account_type_id).unwrap(),
                ),
                guest_users: None,
                created: record.created.into(),
                updated: match record.updated {
                    None => None,
                    Some(res) => Some(DateTime::from(res)),
                },
            })),
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return Err(updating_err(
                        format!("Invalid primary key: {:?}", account_id),
                        None,
                        None,
                    ));
                };

                return Err(updating_err(
                    format!(
                        "Unexpected error detected on update record: {}",
                        err
                    ),
                    Some(false),
                    None,
                ));
            }
        }
    }
}
