use crate::{
    prisma::{
        guest_user as guest_user_model,
        guest_user_on_account as guest_user_on_account_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use log::debug;
use myc_core::domain::{
    dtos::{
        email::Email,
        guest::{GuestRole, GuestUser, Permissions},
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestUserFetching,
};
use mycelium_base::{
    dtos::Parent,
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
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
        // ? Build client
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
        // ? Build and execute the database query
        // ? -------------------------------------------------------------------

        let response = client
            .guest_user()
            .to_owned()
            .find_many(vec![guest_user_model::accounts::some(vec![
                guest_user_on_account_model::account_id::equals(
                    account_id.to_string(),
                ),
            ])])
            .include(guest_user_model::include!({ guest_role }))
            .exec()
            .await
            .unwrap();

        debug!("Guest Record from Account ID: {:?}", response);

        let records: Vec<GuestUser> = response
            .iter()
            .map(|record| GuestUser {
                id: Some(Uuid::parse_str(&record.id).unwrap()),
                email: Email::from_string(record.email.to_owned()).unwrap(),
                guest_role: Parent::Record(GuestRole {
                    id: Some(Uuid::parse_str(&record.guest_role.id).unwrap()),
                    name: record.guest_role.name.to_owned(),
                    description: record.guest_role.description.to_owned(),
                    role: Parent::Id(
                        Uuid::parse_str(&record.guest_role.role_id).unwrap(),
                    ),
                    permissions: record
                        .guest_role
                        .permissions
                        .to_owned()
                        .into_iter()
                        .map(|i| Permissions::from_i32(i))
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

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        if records.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(records))
    }
}
