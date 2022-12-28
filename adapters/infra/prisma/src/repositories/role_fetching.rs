use crate::{prisma::role as role_model, repositories::connector::get_client};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use core::panic;
use myc_core::domain::{dtos::role::RoleDTO, entities::RoleFetching};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RoleFetching)]
pub struct RoleFetchingSqlDbRepository {}

#[async_trait]
impl RoleFetching for RoleFetchingSqlDbRepository {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<RoleDTO, Uuid>, MappedErrors> {
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
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client
            .role()
            .find_unique(role_model::id::equals(id.to_owned().to_string()))
            .exec()
            .await
        {
            Err(err) => {
                return Err(creation_err(
                    format!("Unexpected error on parse user email: {:?}", err,),
                    None,
                    None,
                ))
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(id))),
                Some(record) => Ok(FetchResponseKind::Found(RoleDTO {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    description: record.description.to_owned(),
                })),
            },
        }
    }

    // ? -----------------------------------------------------------------------
    // ! Not implemented structural methods
    // ? -----------------------------------------------------------------------

    async fn list(
        &self,
        _: String,
    ) -> Result<FetchManyResponseKind<RoleDTO>, MappedErrors> {
        panic!("Not implemented list method of RoleFetchingSqlDbRepository.")
    }
}
