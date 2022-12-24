use crate::{
    prisma::account as account_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::account::AccountDTO,
    entities::shared::account_fetching::AccountFetching,
};
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountFetching)]
pub struct AccountFetchingSqlDbRepository {}

#[async_trait]
impl AccountFetching for AccountFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<AccountDTO, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(creation_err(
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
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .account()
            .find_unique(account_model::id::equals(id.to_owned().to_string()))
            .exec()
            .await
        {
            Err(err) => {
                return Err(creation_err(
                    format!("Unexpected error on parse user email: {:?}", err,),
                    None,
                    None,
                ))
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(id))),
                Some(record) => Ok(FetchResponseKind::Found(AccountDTO {
                    id: Some(Uuid::from_str(&record.id).unwrap()),
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
            },
        }
    }

    // ? -----------------------------------------------------------------------
    // ! Not implemented structural methods
    // ? -----------------------------------------------------------------------

    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponseKind<AccountDTO>, MappedErrors> {
        self.list(search_term).await
    }
}
