use crate::{
    prisma::{
        guest_user as guest_user_model,
        guest_user_on_account as guest_user_on_account_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        email::Email, guest::Permissions, native_error_codes::NativeErrorCodes,
        profile::LicensedResources,
    },
    entities::LicensedResourcesFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
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
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        let response = client
            .guest_user_on_account()
            .find_many(vec![guest_user_on_account_model::guest_user::is(vec![
                guest_user_model::email::equals(email.get_email()),
            ])])
            .include(guest_user_on_account_model::include!({
                guest_user: select {
                    guest_role: select {
                        id
                        name
                        role: select {
                            name
                        }
                        permissions
                    }
                }
                account: select {
                    name
                    account_type
                    is_default
                }
            }))
            .exec()
            .await
            .unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        let licenses = response
            .into_iter()
            .map(|record| LicensedResources {
                guest_account_id: Uuid::parse_str(
                    &record.account_id.to_owned(),
                )
                .unwrap(),
                guest_account_name: record.account.name.to_owned(),
                guest_account_is_default: record.account.is_default,
                guest_role_id: Uuid::parse_str(
                    &record.guest_user.guest_role.id,
                )
                .unwrap(),
                guest_role_name: record.guest_user.guest_role.to_owned().name,
                role: record
                    .to_owned()
                    .guest_user
                    .guest_role
                    .role
                    .name
                    .to_owned(),
                permissions: record
                    .to_owned()
                    .guest_user
                    .guest_role
                    .permissions
                    .to_owned()
                    .into_iter()
                    .map(|i| Permissions::from_i32(i))
                    .collect(),
            })
            .collect::<Vec<LicensedResources>>();

        if licenses.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(licenses))
    }
}
