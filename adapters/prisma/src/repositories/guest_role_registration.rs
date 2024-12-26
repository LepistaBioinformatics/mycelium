use crate::{
    prisma::{guest_role as guest_role_model, role as role_model},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        guest_role::{GuestRole, Permission},
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestRoleRegistration,
};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use prisma_client_rust::and;
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
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let response = client
            .guest_role()
            .find_first(vec![and![
                guest_role_model::name::equals(guest_role.name.to_owned()),
                guest_role_model::slug::equals(guest_role.slug.to_owned()),
            ]])
            .include(guest_role_model::include!({
                role: select {
                    id
                }
                children
            }))
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let record = record;
                return Ok(GetOrCreateResponseKind::NotCreated(
                    GuestRole {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        slug: record.slug,
                        description: record.description.to_owned(),
                        role: Parent::Id(
                            Uuid::parse_str(&record.role.id).unwrap(),
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
                guest_role.slug.to_owned(),
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
                    guest_role_model::permission::set(
                        guest_role.permission as i32,
                    ),
                ],
            )
            .include(guest_role_model::include!({ children }))
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;

                Ok(GetOrCreateResponseKind::Created(GuestRole {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    slug: record.slug,
                    description: record.description,
                    role: Parent::Id(Uuid::parse_str(&record.role_id).unwrap()),
                    children: match record.children.len() {
                        0 => None,
                        _ => Some(Children::Ids(
                            record
                                .children
                                .into_iter()
                                .map(|i| {
                                    Uuid::parse_str(&i.child_role_id).unwrap()
                                })
                                .collect(),
                        )),
                    },
                    permission: Permission::from_i32(record.permission),
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
