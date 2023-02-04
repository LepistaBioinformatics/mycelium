use crate::{prisma::role as role_model, repositories::connector::get_client};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use myc_core::domain::entities::RoleDeletion;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RoleDeletion)]
pub struct RoleDeletionSqlDbRepository {}

#[async_trait]
impl RoleDeletion for RoleDeletionSqlDbRepository {
    async fn delete(
        &self,
        role_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
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
            .role()
            .delete(role_model::id::equals(role_id.to_string()))
            .exec()
            .await
        {
            Err(err) => {
                Ok(DeletionResponseKind::NotDeleted(role_id, err.to_string()))
            }
            Ok(_) => Ok(DeletionResponseKind::Deleted),
        }
    }
}
