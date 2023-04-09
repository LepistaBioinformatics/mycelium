use crate::{prisma::role as role_model, repositories::connector::get_client};

use async_trait::async_trait;
use clean_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use myc_core::domain::{dtos::role::Role, entities::RoleUpdating};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RoleUpdating)]
pub struct RoleUpdatingSqlDbRepository {}

#[async_trait]
impl RoleUpdating for RoleUpdatingSqlDbRepository {
    async fn update(
        &self,
        role: Role,
    ) -> Result<UpdatingResponseKind<Role>, MappedErrors> {
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

        let role_id = match role.id {
            None => {
                return Err(updating_err(
                    String::from("Unable to update user. Invalid record ID"),
                    None,
                    None,
                ))
            }
            Some(res) => res,
        };

        let response = client
            .role()
            .update(
                role_model::id::equals(role_id.to_string()),
                vec![
                    role_model::name::set(role.name),
                    role_model::description::set(role.description),
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => Ok(UpdatingResponseKind::Updated(Role {
                id: Some(Uuid::parse_str(&record.id).unwrap()),
                name: record.name,
                description: record.description.to_owned(),
            })),
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return Err(updating_err(
                        format!("Invalid primary key: {:?}", role_id),
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
