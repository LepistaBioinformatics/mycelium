use crate::{
    prisma::guest_user_on_account as guest_user_on_account_model,
    repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use myc_core::domain::entities::GuestUserDeletion;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserDeletion)]
pub struct GuestUserDeletionSqlDbRepository {}

#[async_trait]
impl GuestUserDeletion for GuestUserDeletionSqlDbRepository {
    async fn delete(
        &self,
        guest_user_id: Uuid,
        account_id: Uuid,
    ) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(deletion_err(
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
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        match client
            .guest_user_on_account()
            .delete(guest_user_on_account_model::guest_user_id_account_id(
                guest_user_id.to_string(),
                account_id.to_string(),
            ))
            .exec()
            .await
        {
            Err(err) => Ok(DeletionResponseKind::NotDeleted(
                (guest_user_id, account_id),
                err.to_string(),
            )),
            Ok(_) => Ok(DeletionResponseKind::Deleted),
        }
    }
}
