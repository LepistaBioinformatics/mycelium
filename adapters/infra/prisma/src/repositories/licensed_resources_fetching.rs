use crate::{
    prisma::{guest_role as guest_role_model, guest_user as guest_user_model},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use log::debug;
use myc_core::domain::{
    dtos::{email::Email, guest::PermissionsType, profile::LicensedResources},
    entities::LicensedResourcesFetching,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component, Debug)]
#[shaku(interface = LicensedResourcesFetching)]
pub struct LicensedResourcesFetchingSqlDbRepository {}

#[async_trait]
impl LicensedResourcesFetching for LicensedResourcesFetchingSqlDbRepository {
    async fn list(
        &self,
        email: Email,
    ) -> Result<FetchManyResponseKind<LicensedResources>, MappedErrors> {
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
            .guest_role()
            .find_many(vec![guest_role_model::guest_users::every(vec![
                guest_user_model::email::equals(email.get_email()),
            ])])
            .include(guest_role_model::include!({
                role
                guest_users: select {
                    accounts: select {
                        account_id
                    }
                }
            }))
            .exec()
            .await
            .unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        debug!("Raw Licensed Resources: {:?}", response);

        let licenses = response
            .into_iter()
            .map(|record| {
                record
                    .guest_users
                    .into_iter()
                    .map(move |guest_user| {
                        guest_user
                            .accounts
                            .into_iter()
                            .map(|account| LicensedResources {
                                guest_account_id: Uuid::parse_str(
                                    &account.account_id,
                                )
                                .unwrap(),
                                role: record.role.name.to_owned(),
                                permissions: record
                                    .permissions
                                    .to_owned()
                                    .into_iter()
                                    .map(|i| PermissionsType::from_i32(i))
                                    .collect(),
                            })
                            .collect::<Vec<LicensedResources>>()
                    })
                    .flatten()
            })
            .flatten()
            .collect::<Vec<LicensedResources>>();

        debug!("Parsed Licensed Resources: {:?}", licenses);

        if licenses.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(licenses))
    }
}
