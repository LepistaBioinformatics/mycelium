use crate::{prisma::role as role_model, repositories::connector::get_client};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{native_error_codes::NativeErrorCodes, role::Role},
    entities::RoleFetching,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
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
    ) -> Result<FetchResponseKind<Role, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
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
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => match res {
                None => Ok(FetchResponseKind::NotFound(Some(id))),
                Some(record) => Ok(FetchResponseKind::Found(Role {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    slug: record.slug,
                    description: record.description.to_owned(),
                })),
            },
        }
    }

    async fn list(
        &self,
        name: Option<String>,
    ) -> Result<FetchManyResponseKind<Role>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build list query statement
        // ? -------------------------------------------------------------------

        let mut query_stmt = vec![];

        if name.is_some() {
            query_stmt.push(role_model::name::contains(name.unwrap()))
        }

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client.role().find_many(query_stmt).exec().await {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err
                ))
                .as_error()
            }
            Ok(res) => {
                let response = res
                    .into_iter()
                    .map(|record| Role {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        slug: record.slug,
                        description: record.description.to_owned(),
                    })
                    .collect::<Vec<Role>>();

                if response.len() == 0 {
                    return Ok(FetchManyResponseKind::NotFound);
                }

                Ok(FetchManyResponseKind::Found(response))
            }
        }
    }
}
