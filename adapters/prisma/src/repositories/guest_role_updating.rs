use crate::{
    prisma::guest_role as guest_role_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::UpdatingResponseKind,
    utils::errors::{factories::updating_err, MappedErrors},
};
use myc_core::domain::{
    dtos::guest::{GuestRole, PermissionsType},
    entities::GuestRoleUpdating,
};
use prisma_client_rust::prisma_errors::query_engine::RecordNotFound;
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleUpdating)]
pub struct GuestRoleUpdatingSqlDbRepository {}

#[async_trait]
impl GuestRoleUpdating for GuestRoleUpdatingSqlDbRepository {
    async fn update(
        &self,
        user_role: GuestRole,
    ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code("MYC00001".to_string())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to update record
        // ? -------------------------------------------------------------------

        let user_role_id = match user_role.id {
            None => {
                return updating_err(String::from(
                    "Unable to update account. Invalid record ID",
                ))
                .with_exp_false()
                .as_error()
            }
            Some(res) => res,
        };

        let response = client
            .guest_role()
            .update(
                guest_role_model::id::equals(user_role_id.to_string()),
                vec![
                    guest_role_model::name::set(user_role.name),
                    guest_role_model::description::set(user_role.description),
                    guest_role_model::permissions::set(
                        user_role
                            .permissions
                            .into_iter()
                            .map(|i| i as i32)
                            .collect::<Vec<i32>>(),
                    ),
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => Ok(UpdatingResponseKind::Updated(GuestRole {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                role: ParentEnum::Id(Uuid::from_str(&record.role_id).unwrap()),
                permissions: record
                    .permissions
                    .into_iter()
                    .map(|i| PermissionsType::from_i32(i))
                    .collect(),
            })),
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        user_role_id
                    ))
                    .with_exp_false()
                    .as_error();
                };

                return updating_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        }
    }
}
