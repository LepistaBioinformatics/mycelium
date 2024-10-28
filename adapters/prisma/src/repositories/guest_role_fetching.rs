use crate::{
    prisma::{
        guest_role as guest_role_model,
        guest_role_children as guest_role_children_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        guest_role::{GuestRole, Permission},
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestRoleFetching,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
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
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        let (guest_role_response, children_response) = match client
            ._transaction()
            .run(|client| async move {
                let record = client
                    .guest_role()
                    .find_unique(guest_role_model::id::equals(
                        id.to_owned().to_string(),
                    ))
                    .include(guest_role_model::include!({ children }))
                    .exec()
                    .await?;

                client
                    .guest_role_children()
                    .find_many(vec![
                        guest_role_children_model::parent_id::equals(
                            id.to_owned().to_string(),
                        ),
                    ])
                    .include(guest_role_children_model::include!({
                        child_role
                    }))
                    .exec()
                    .await
                    .map(|children| (record, children))
            })
            .await
        {
            Ok(res) => res,
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err,
                ))
                .with_exp_true()
                .as_error()
            }
        };

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        match guest_role_response {
            Some(record) => Ok(FetchResponseKind::Found(GuestRole {
                id: Some(Uuid::parse_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                role: Parent::Id(Uuid::parse_str(&record.role_id).unwrap()),
                children: match record.children.len() {
                    0 => None,
                    _ => Some(Children::Records(
                        children_response
                            .into_iter()
                            .map(|i| {
                                let child_role = i.child_role;

                                GuestRole {
                                    id: Some(
                                        Uuid::parse_str(&child_role.id)
                                            .unwrap(),
                                    ),
                                    name: child_role.name,
                                    description: child_role.description,
                                    role: Parent::Id(
                                        Uuid::parse_str(&child_role.role_id)
                                            .unwrap(),
                                    ),
                                    children: None,
                                    permission: Permission::from_i32(
                                        child_role.permission,
                                    ),
                                }
                            })
                            .collect(),
                    )),
                },
                permission: Permission::from_i32(record.permission),
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

        match client
            .guest_role()
            .find_many(query_stmt)
            .include(guest_role_model::include!({ children }))
            .exec()
            .await
        {
            Err(err) => {
                return fetching_err(format!(
                    "Unexpected error on parse user email: {:?}",
                    err,
                ))
                .with_exp_true()
                .as_error()
            }
            Ok(res) => {
                let response = res
                    .into_iter()
                    .map(|record| GuestRole {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description,
                        role: Parent::Id(
                            Uuid::parse_str(&record.role_id).unwrap(),
                        ),
                        children: match record.children.len() {
                            0 => None,
                            _ => Some(Children::Ids(
                                record
                                    .children
                                    .into_iter()
                                    .map(|i| {
                                        Uuid::parse_str(&i.child_role_id)
                                            .unwrap()
                                    })
                                    .collect(),
                            )),
                        },
                        permission: Permission::from_i32(record.permission),
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
