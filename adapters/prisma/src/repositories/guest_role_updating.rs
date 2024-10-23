use crate::{
    prisma::guest_role as guest_role_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        guest_role::{GuestRole, Permissions},
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestRoleUpdating,
};
use mycelium_base::{
    dtos::Parent,
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
                role: Parent::Id(Uuid::from_str(&record.role_id).unwrap()),
                children: match record.children.len() {
                    0 => None,
                    _ => Some(
                        record
                            .children
                            .into_iter()
                            .map(|i| Uuid::parse_str(&i).unwrap())
                            .collect(),
                    ),
                },
                permissions: record
                    .permissions
                    .into_iter()
                    .map(|i| Permissions::from_i32(i))
                    .collect(),
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

    async fn insert_role_children(
        &self,
        role_id: Uuid,
        child_id: Uuid,
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
        let response = client
            ._transaction()
            .run(|client| async move {
                let guest_role = client
                    .guest_role()
                    .find_unique(guest_role_model::id::equals(
                        role_id.to_string(),
                    ))
                    .select(guest_role_model::select!({ children }))
                    .exec()
                    .await?;

                let children = if let Some(data) = guest_role {
                    let mut children = data.children;

                    if !children.contains(&child_id.to_string()) {
                        children.push(child_id.to_string());
                    }

                    children
                } else {
                    vec![child_id.to_string()]
                };

                client
                    .guest_role()
                    .update(
                        guest_role_model::id::equals(role_id.to_string()),
                        vec![guest_role_model::children::set(children)],
                    )
                    .exec()
                    .await
            })
            .await;

        match response {
            Ok(record) => Ok(UpdatingResponseKind::Updated(GuestRole {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                role: Parent::Id(Uuid::from_str(&record.role_id).unwrap()),
                children: match record.children.len() {
                    0 => None,
                    _ => Some(
                        record
                            .children
                            .into_iter()
                            .map(|i| Uuid::parse_str(&i).unwrap())
                            .collect(),
                    ),
                },
                permissions: record
                    .permissions
                    .into_iter()
                    .map(|i| Permissions::from_i32(i))
                    .collect(),
            })),
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

    async fn remove_role_children(
        &self,
        role_id: Uuid,
        child_id: Uuid,
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
        let response = client
            ._transaction()
            .run(|client| async move {
                let guest_role = client
                    .guest_role()
                    .find_unique(guest_role_model::id::equals(
                        role_id.to_string(),
                    ))
                    .select(guest_role_model::select!({ children }))
                    .exec()
                    .await?;

                let children = if let Some(data) = guest_role {
                    let mut children = data.children;

                    if let Some(index) =
                        children.iter().position(|i| i == &child_id.to_string())
                    {
                        children.remove(index);
                    }

                    children
                } else {
                    vec![child_id.to_string()]
                };

                client
                    .guest_role()
                    .update(
                        guest_role_model::id::equals(role_id.to_string()),
                        vec![guest_role_model::children::set(children)],
                    )
                    .exec()
                    .await
            })
            .await;

        match response {
            Ok(record) => Ok(UpdatingResponseKind::Updated(GuestRole {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                name: record.name,
                description: record.description,
                role: Parent::Id(Uuid::from_str(&record.role_id).unwrap()),
                children: match record.children.len() {
                    0 => None,
                    _ => Some(
                        record
                            .children
                            .into_iter()
                            .map(|i| Uuid::parse_str(&i).unwrap())
                            .collect(),
                    ),
                },
                permissions: record
                    .permissions
                    .into_iter()
                    .map(|i| Permissions::from_i32(i))
                    .collect(),
            })),
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
