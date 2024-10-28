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
    entities::GuestRoleUpdating,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
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
                .with_code(NativeErrorCodes::MYC00001)
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
                .with_exp_true()
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
                    guest_role_model::permission::set(
                        user_role.permission as i32,
                    ),
                ],
            )
            .include(guest_role_model::include!({ children }))
            .exec()
            .await;

        match response {
            Ok(record) => Ok(UpdatingResponseKind::Updated(GuestRole {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                role: Parent::Id(Uuid::from_str(&record.role_id).unwrap()),
                children: match record.children.len() {
                    0 => None,
                    _ => Some(Children::Ids(
                        record
                            .children
                            .into_iter()
                            .map(|i| Uuid::parse_str(&i.child_role_id).unwrap())
                            .collect(),
                    )),
                },
                permission: Permission::from_i32(record.permission),
            })),
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid primary key: {:?}",
                        user_role_id
                    ))
                    .with_exp_true()
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

    async fn insert_role_child(
        &self,
        role_id: Uuid,
        child_id: Uuid,
    ) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to update record
        // ? -------------------------------------------------------------------

        let response = client
            ._transaction()
            .run(|client| async move {
                client
                    .guest_role_children()
                    .create(
                        guest_role_model::id::equals(role_id.to_string()),
                        guest_role_model::id::equals(child_id.to_string()),
                        vec![],
                    )
                    .exec()
                    .await?;

                client
                    .guest_role()
                    .find_unique(guest_role_model::id::equals(
                        role_id.to_string(),
                    ))
                    .include(guest_role_model::include!({ children }))
                    .exec()
                    .await
            })
            .await;

        match response {
            Ok(record) => {
                if let Some(record) = record {
                    return Ok(UpdatingResponseKind::Updated(Some(
                        GuestRole {
                            id: Some(Uuid::from_str(&record.id).unwrap()),
                            name: record.name,
                            description: record.description,
                            role: Parent::Id(
                                Uuid::from_str(&record.role_id).unwrap(),
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
                        },
                    )));
                };

                Ok(UpdatingResponseKind::NotUpdated(
                    None,
                    format!("Invalid guest-role key: {:?}", role_id),
                ))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid guest-role key: {:?}",
                        role_id
                    ))
                    .with_exp_true()
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

    async fn remove_role_child(
        &self,
        role_id: Uuid,
        child_id: Uuid,
    ) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return updating_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to update record
        // ? -------------------------------------------------------------------
        let response = client
            ._transaction()
            .run(|client| async move {
                client
                    .guest_role_children()
                    .delete(guest_role_children_model::parent_id_child_role_id(
                        role_id.to_string(),
                        child_id.to_string(),
                    ))
                    .exec()
                    .await?;

                client
                    .guest_role()
                    .find_unique(guest_role_model::id::equals(
                        role_id.to_string(),
                    ))
                    .include(guest_role_model::include!({ children }))
                    .exec()
                    .await
            })
            .await;

        match response {
            Ok(record) => {
                if let Some(record) = record {
                    return Ok(UpdatingResponseKind::Updated(Some(
                        GuestRole {
                            id: Some(Uuid::from_str(&record.id).unwrap()),
                            name: record.name,
                            description: record.description,
                            role: Parent::Id(
                                Uuid::from_str(&record.role_id).unwrap(),
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
                        },
                    )));
                };

                Ok(UpdatingResponseKind::NotUpdated(
                    None,
                    format!("Invalid guest-role key: {:?}", role_id),
                ))
            }
            Err(err) => {
                if err.is_prisma_error::<RecordNotFound>() {
                    return updating_err(format!(
                        "Invalid guest-role key: {:?}",
                        role_id
                    ))
                    .with_exp_true()
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
