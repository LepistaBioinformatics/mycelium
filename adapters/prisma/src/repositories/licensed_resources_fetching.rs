use crate::repositories::connector::get_client;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        email::Email, guest::Permissions, native_error_codes::NativeErrorCodes,
        profile::LicensedResources, related_accounts::RelatedAccounts,
    },
    entities::LicensedResourcesFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use prisma_client_rust::{raw, PrismaValue};
use serde::Deserialize;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component, Debug)]
#[shaku(interface = LicensedResourcesFetching)]
pub struct LicensedResourcesFetchingSqlDbRepository {}

#[derive(Deserialize, Debug)]
struct LicensedResourceRow {
    acc_id: String,
    acc_name: String,
    is_acc_std: bool,
    gr_id: String,
    gr_name: String,
    gr_perms: Vec<i32>,
    rl_name: String,
}

#[async_trait]
impl LicensedResourcesFetching for LicensedResourcesFetchingSqlDbRepository {
    async fn list(
        &self,
        email: Email,
        related_accounts: Option<RelatedAccounts>,
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
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        let response: Vec<LicensedResourceRow> = match client
            ._query_raw(raw!(
                "SELECT * FROM licensed_resources WHERE gu_email = {}",
                PrismaValue::String(email.get_email())
            ))
            .exec()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                return fetching_err(e.to_string())
                    .with_code(NativeErrorCodes::MYC00001)
                    .as_error()
            }
        };

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        let licenses = response
            .into_iter()
            .map(|record| LicensedResources {
                acc_id: Uuid::parse_str(&record.acc_id.to_owned()).unwrap(),
                acc_name: record.acc_name.to_owned(),
                is_acc_std: record.is_acc_std,
                guest_role_id: Uuid::parse_str(&record.gr_id).unwrap(),
                guest_role_name: record.gr_name,
                role: record.rl_name,
                perms: record
                    .gr_perms
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
