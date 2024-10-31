use crate::repositories::connector::get_client;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        email::Email, guest_role::Permission,
        native_error_codes::NativeErrorCodes, profile::LicensedResources,
        related_accounts::RelatedAccounts, route_type::PermissionedRoles,
    },
    entities::LicensedResourcesFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use prisma_client_rust::{PrismaValue, Raw};
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
    tenant_id: String,
    is_acc_std: bool,
    gr_id: String,
    gr_name: String,
    gr_perm: i32,
    rl_name: String,
}

#[async_trait]
impl LicensedResourcesFetching for LicensedResourcesFetchingSqlDbRepository {
    async fn list(
        &self,
        email: Email,
        roles: Option<Vec<String>>,
        permissioned_roles: Option<PermissionedRoles>,
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

        let mut _role = roles.clone();
        if roles.is_some() && permissioned_roles.is_some() {
            _role = None;
        }

        let mut query =
            vec!["SELECT * FROM licensed_resources WHERE gu_email = {}"];

        let mut params = vec![PrismaValue::String(email.get_email())];

        if let Some(related_accounts) = related_accounts {
            if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
                query.push("AND acc_id = ANY({})");
                params.push(PrismaValue::List(
                    ids.into_iter()
                        .map(|i| PrismaValue::Uuid(i))
                        .collect::<Vec<PrismaValue>>(),
                ));
            }
        };

        if let Some(roles) = _role {
            query.push("AND rl_name = ANY({})");
            params.push(PrismaValue::List(
                roles
                    .into_iter()
                    .map(|i| PrismaValue::String(i.to_string()))
                    .collect::<Vec<PrismaValue>>(),
            ));
        };

        let query = if let Some(permissioned_roles) = permissioned_roles {
            let mut _query =
                query.iter().map(|i| i.to_string()).collect::<Vec<String>>();

            let statement = permissioned_roles.iter().fold(
                String::new(),
                |acc, (role, permission)| {
                    format!(
                        "{}(rl_name = '{}' AND gr_perm >= {}) OR ",
                        acc,
                        role,
                        permission.to_owned() as i64
                    )
                },
            );

            let statement = statement.trim_end_matches(" OR ").to_owned();
            let binding = format!("AND ({})", statement.clone());

            _query.push(binding);
            _query.iter().map(|i| i.to_owned()).collect::<Vec<_>>()
        } else {
            query.iter().map(|i| i.to_string()).collect::<Vec<_>>()
        };

        let join_query = query.join(" ");

        let response: Vec<LicensedResourceRow> = match client
            ._query_raw(Raw::new(join_query.as_str(), params))
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
                tenant_id: Uuid::parse_str(&&record.tenant_id.to_owned())
                    .unwrap(),
                acc_name: record.acc_name.to_owned(),
                is_acc_std: record.is_acc_std,
                guest_role_id: Uuid::parse_str(&record.gr_id).unwrap(),
                guest_role_name: record.gr_name,
                role: record.rl_name,
                perm: Permission::from_i32(record.gr_perm),
            })
            .collect::<Vec<LicensedResources>>();

        if licenses.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(licenses))
    }
}
