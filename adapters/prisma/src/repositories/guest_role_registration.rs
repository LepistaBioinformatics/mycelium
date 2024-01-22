use crate::{
    prisma::{guest_role as guest_role_model, role as role_model},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        guest::{GuestRole, Permissions},
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestRoleRegistration,
};
use mycelium_base::{
    dtos::Parent,
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestRoleRegistration)]
pub struct GuestRoleRegistrationSqlDbRepository {}

#[async_trait]
impl GuestRoleRegistration for GuestRoleRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        guest_role: GuestRole,
    ) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
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

        let response = client
            .guest_role()
            .find_first(vec![guest_role_model::name::equals(
                guest_role.name.to_owned(),
            )])
            .include(guest_role_model::include!({ role: select { id } }))
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let record = record;
                return Ok(GetOrCreateResponseKind::NotCreated(
                    GuestRole {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description.to_owned(),
                        role: Parent::Id(
                            Uuid::parse_str(&record.role.id).unwrap(),
                        ),
                        permissions: record
                            .permissions
                            .into_iter()
                            .map(|i| Permissions::from_i32(i))
                            .collect(),
                    },
                    String::from("Account type already exists"),
                ));
            }
            None => (),
        };

        // ? -------------------------------------------------------------------
        // ? Build create part of the get-or-create
        // ? -------------------------------------------------------------------

        let response = client
            .guest_role()
            .create(
                guest_role.name.to_owned(),
                role_model::id::equals(match guest_role.role {
                    Parent::Id(id) => id.to_string(),
                    Parent::Record(record) => match record.id {
                        None => {
                            return creation_err(format!(
                                "Role ID not available: {:?}",
                                guest_role.id.to_owned(),
                            ))
                            .with_exp_true()
                            .as_error()
                        }
                        Some(id) => id.to_string(),
                    },
                }),
                vec![
                    guest_role_model::description::set(guest_role.description),
                    guest_role_model::permissions::set(
                        guest_role
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
            Ok(record) => {
                let record = record;

                Ok(GetOrCreateResponseKind::Created(GuestRole {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    description: record.description,
                    role: Parent::Id(Uuid::parse_str(&record.role_id).unwrap()),
                    permissions: record
                        .permissions
                        .into_iter()
                        .map(|i| Permissions::from_i32(i))
                        .collect(),
                }))
            }
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    err
                ))
                .with_exp_true()
                .as_error();
            }
        }
    }
}
