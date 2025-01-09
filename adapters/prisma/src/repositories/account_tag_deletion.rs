use super::connector::get_client;
use crate::prisma::account_tags as result_tags_model;

use async_trait::async_trait;
use myc_core::domain::entities::AccountTagDeletion;
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountTagDeletion)]
pub struct AccountTagDeletionSqlDbRepository {}

#[async_trait]
impl AccountTagDeletion for AccountTagDeletionSqlDbRepository {
    // ? ----------------------------------------------------------------------
    // ? Abstract methods implementation
    // ? ----------------------------------------------------------------------

    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
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

        match client
            .account_tags()
            .delete(result_tags_model::id::equals(id.to_owned().to_string()))
            .exec()
            .await
        {
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return deletion_err(format!(
                        "Invalid primary key: {:?}",
                        id.to_string()
                    ))
                    .as_error();
                };

                return deletion_err(format!(
                    "Unexpected error detected on delete record: {}",
                    err
                ))
                .as_error();
            }
            Ok(_) => Ok(DeletionResponseKind::Deleted),
        }
    }
}
