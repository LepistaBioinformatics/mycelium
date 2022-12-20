use crate::{
    adapters::repositories::sql_db::connector::get_client,
    domain::{
        dtos::guest::{GuestRoleDTO, PermissionsType},
        entities::manager::guest_role_registration::GuestRoleRegistration,
    },
};

use async_trait::async_trait;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use myc_prisma::prisma::guest_role as guest_role_model;
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
        guest_role: GuestRoleDTO,
    ) -> Result<GetOrCreateResponseKind<GuestRoleDTO>, MappedErrors> {
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
            .guest_role()
            .find_first(vec![guest_role_model::name::equals(
                guest_role.name.to_owned(),
            )])
            .include(guest_role_model::include!({ role_id }))
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let record = record;
                return Ok(GetOrCreateResponseKind::NotCreated(
                    GuestRoleDTO {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description,
                        role: record.role_id,
                        permissions: record
                            .permissions
                            .into_iter()
                            .map(|i| PermissionsType::from_i32(i))
                            .collect(),
                        account: None,
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
            .account_type()
            .create(
                guest_role.name.to_owned(),
                guest_role.description.to_owned(),
                vec![
                    guest_role_model::role_id::set(match guest_role.role {
                        ParentEnum::id(id) => id.to_string(),
                        ParentEnum::Record(record) => match record.id {
                            None => {
                                return Err(creation_err(
                                    format!(
                                        "Role ID not available: {}",
                                        guest_role.to_owned(),
                                    ),
                                    None,
                                    None,
                                ))
                            }
                            Some(id) => id,
                        },
                    }),
                    guest_role_model::permissions::set(guest_role.permissions),
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;

                Ok(GetOrCreateResponseKind::Created(GuestRoleDTO {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    description: record.description,
                    role: record.role_id,
                    permissions: record
                        .permissions
                        .into_iter()
                        .map(|i| PermissionsType::from_i32(i))
                        .collect(),
                    account: None,
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
}
