use crate::{
    prisma::{
        guest_user as guest_user_model,
        guest_user_on_account as guest_user_on_account_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{
        email::Email,
        guest::{GuestRole, GuestUser, PermissionsType},
    },
    entities::GuestUserFetching,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component, Debug)]
#[shaku(interface = GuestUserFetching)]
pub struct GuestUserFetchingSqlDbRepository {}

#[async_trait]
impl GuestUserFetching for GuestUserFetchingSqlDbRepository {
    async fn list(
        &self,
        account_id: Uuid,
    ) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors> {
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
            .guest_user()
            .find_many(vec![guest_user_model::accounts::every(vec![
                guest_user_on_account_model::account_id::equals(
                    account_id.to_string(),
                ),
            ])])
            .include(guest_user_model::include!({ role }))
            .exec()
            .await
            .unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        let records = response
            .iter()
            .map(|record| GuestUser {
                id: Some(Uuid::parse_str(&record.id).unwrap()),
                email: Email::from_string(record.email.to_owned()).unwrap(),
                guest_role: ParentEnum::Record(GuestRole {
                    id: Some(Uuid::parse_str(&record.role.id).unwrap()),
                    name: record.role.name.to_owned(),
                    description: record.role.description.to_owned(),
                    role: ParentEnum::Id(
                        Uuid::parse_str(&record.role.id).unwrap(),
                    ),
                    permissions: record
                        .role
                        .permissions
                        .to_owned()
                        .into_iter()
                        .map(|i| PermissionsType::from_i32(i))
                        .collect(),
                }),
                created: record.created.into(),
                updated: match record.updated {
                    None => None,
                    Some(res) => Some(DateTime::from(res)),
                },
                accounts: None,
            })
            .collect::<Vec<GuestUser>>();

        if records.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(records))
    }
}
