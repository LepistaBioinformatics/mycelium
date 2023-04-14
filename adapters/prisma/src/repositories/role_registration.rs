use crate::{prisma::role as role_model, repositories::connector::get_client};

use async_trait::async_trait;
use clean_base::{
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{factories::creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, role::Role},
    entities::RoleRegistration,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RoleRegistration)]
pub struct RoleRegistrationSqlDbRepository {}

#[async_trait]
impl RoleRegistration for RoleRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        role: Role,
    ) -> Result<GetOrCreateResponseKind<Role>, MappedErrors> {
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
            .role()
            .find_first(vec![role_model::name::equals(role.name.to_owned())])
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let record = record;
                return Ok(GetOrCreateResponseKind::NotCreated(
                    Role {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description.to_owned(),
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
            .role()
            .create(role.name.to_owned(), role.description.to_owned(), vec![])
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;

                Ok(GetOrCreateResponseKind::Created(Role {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    description: record.description.to_owned(),
                }))
            }
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    err
                ))
                .as_error();
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ! NOT IMPLEMENTED METHOD
    // ? -----------------------------------------------------------------------

    async fn create(
        &self,
        _: Role,
    ) -> Result<CreateResponseKind<Role>, MappedErrors> {
        panic!(
            "Not implemented method create of RoleRegistrationSqlDbRepository."
        )
    }
}
