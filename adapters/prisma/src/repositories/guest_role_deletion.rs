use crate::{
    prisma::guest_role as guest_role_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::GuestRoleDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleDeletion)]
pub struct GuestRoleDeletionSqlDbRepository {}

#[async_trait]
impl GuestRoleDeletion for GuestRoleDeletionSqlDbRepository {
    async fn delete(
        &self,
        user_role_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return deletion_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        match client
            .guest_role()
            .delete(guest_role_model::id::equals(user_role_id.to_string()))
            .exec()
            .await
        {
            Err(err) => Ok(DeletionResponseKind::NotDeleted(
                user_role_id,
                err.to_string(),
            )),
            Ok(_) => Ok(DeletionResponseKind::Deleted),
        }
    }
}
