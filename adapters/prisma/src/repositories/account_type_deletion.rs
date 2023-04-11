use crate::{
    prisma::account_type as account_type_model,
    repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::deletion_err, MappedErrors},
};
use myc_core::domain::{
    dtos::account::AccountType, entities::AccountTypeDeletion,
};
use shaku::Component;
use std::process::id as process_id;

#[derive(Component)]
#[shaku(interface = AccountTypeDeletion)]
pub struct AccountTypeDeletionSqlDbRepository {}

#[async_trait]
impl AccountTypeDeletion for AccountTypeDeletionSqlDbRepository {
    async fn delete(
        &self,
        account_type: AccountType,
    ) -> Result<DeletionResponseKind<AccountType>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return deletion_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        match client
            .account_type()
            .delete(account_type_model::id::equals(match account_type.id {
                None => {
                    return deletion_err(String::from(
                        "Could not delete account type without ID.",
                    ))
                    .as_error()
                }
                Some(id) => id.to_string(),
            }))
            .exec()
            .await
        {
            Err(err) => Ok(DeletionResponseKind::NotDeleted(
                account_type,
                err.to_string(),
            )),
            Ok(_) => Ok(DeletionResponseKind::Deleted),
        }
    }
}
