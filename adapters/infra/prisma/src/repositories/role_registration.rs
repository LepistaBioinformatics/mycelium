use crate::{prisma::role as role_model, repositories::connector::get_client};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::{dtos::role::RoleDTO, entities::RoleRegistration};
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
        role: RoleDTO,
    ) -> Result<GetOrCreateResponseKind<RoleDTO>, MappedErrors> {
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
                    RoleDTO {
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

                Ok(GetOrCreateResponseKind::Created(RoleDTO {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    description: record.description.to_owned(),
                }))
            }
            Err(err) => {
                return Err(creation_err(
                    format!(
                        "Unexpected error detected on create record: {}",
                        err
                    ),
                    None,
                    None,
                ));
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ! NOT IMPLEMENTED METHOD
    // ? -----------------------------------------------------------------------

    async fn create(
        &self,
        _: RoleDTO,
    ) -> Result<CreateResponseKind<RoleDTO>, MappedErrors> {
        panic!(
            "Not implemented method create of RoleRegistrationSqlDbRepository."
        )
    }
}
