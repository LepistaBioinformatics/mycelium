use crate::{
    prisma::guest_role as guest_role_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{factories::fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{
        guest::{GuestRole, PermissionsType},
        native_error_codes::NativeErrorCodes,
    },
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
    ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Build and execute the database query
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
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
            Some(record) => Ok(FetchResponseKind::Found(GuestRole {
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

    async fn list(
        &self,
        name: Option<String>,
        role_id: Option<Uuid>,
    ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build list query statement
        // ? -------------------------------------------------------------------

        let mut query_stmt = vec![];

        if name.is_some() {
            query_stmt.push(guest_role_model::name::contains(name.unwrap()))
        }

        if role_id.is_some() {
            query_stmt.push(guest_role_model::role_id::equals(
                role_id.unwrap().to_string(),
            ))
        }

        // ? -------------------------------------------------------------------
        // ? Get the user
        // ? -------------------------------------------------------------------

        match client.guest_role().find_many(query_stmt).exec().await {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err,
                ))
                .with_exp_false()
                .as_error()
            }
            Ok(res) => {
                let response = res
                    .into_iter()
                    .map(|record| GuestRole {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description,
                        role: ParentEnum::Id(
                            Uuid::parse_str(&record.role_id).unwrap(),
                        ),
                        permissions: record
                            .permissions
                            .into_iter()
                            .map(|i| PermissionsType::from_i32(i))
                            .collect(),
                    })
                    .collect::<Vec<GuestRole>>();

                if response.len() == 0 {
                    return Ok(FetchManyResponseKind::NotFound);
                }

                Ok(FetchManyResponseKind::Found(response))
            }
        }
    }
}
