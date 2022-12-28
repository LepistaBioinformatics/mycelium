use crate::{
    prisma::guest_role as guest_role_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::guest::{GuestRoleDTO, PermissionsType},
    entities::GuestRoleFetching,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component, Debug)]
#[shaku(interface = GuestRoleFetching)]
pub struct GuestRoleFetchingSqlDbRepository {}

#[async_trait]
impl GuestRoleFetching for GuestRoleFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<GuestRoleDTO, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Build and execute the database query
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(fetching_err(
                    String::from(
                        "Prisma Client error. Could not fetch client.",
                    ),
                    Some(false),
                    None,
                ))
            }
            Some(res) => res,
        };

        let response = client
            .guest_role()
            .find_unique(guest_role_model::id::equals(
                id.to_owned().to_string(),
            ))
            .exec()
            .await
            .unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        match response {
            Some(record) => Ok(FetchResponseKind::Found(GuestRoleDTO {
                id: Some(Uuid::parse_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                role: ParentEnum::Id(Uuid::parse_str(&record.role_id).unwrap()),
                permissions: record
                    .permissions
                    .into_iter()
                    .map(|i| PermissionsType::from_i32(i))
                    .collect(),
            })),
            None => Ok(FetchResponseKind::NotFound(Some(id))),
        }
    }

    // ? -----------------------------------------------------------------------
    // ! NOT IMPLEMENTED METHOD
    // ? -----------------------------------------------------------------------

    async fn list(
        &self,
        _: String,
    ) -> Result<FetchManyResponseKind<GuestRoleDTO>, MappedErrors> {
        panic!(
            "Not implemented list method of GuestRoleFetchingSqlDbRepository."
        )
    }
}
