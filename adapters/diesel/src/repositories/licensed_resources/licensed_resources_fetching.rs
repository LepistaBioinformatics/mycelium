use crate::{
    models::config::DbPoolProvider,
    schema::{owner_on_tenant::dsl as owner_dsl, user::dsl as user_dsl},
};

use async_trait::async_trait;
use chrono::{Local, NaiveDateTime};
use diesel::{
    prelude::*,
    sql_types::{Bool, Integer, Nullable, Text},
    RunQueryDsl,
};
use myc_core::domain::{
    dtos::{
        email::Email,
        guest_role::Permission,
        native_error_codes::NativeErrorCodes,
        profile::{LicensedResource, TenantOwnership},
        related_accounts::RelatedAccounts,
        route_type::PermissionedRoles,
    },
    entities::LicensedResourcesFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use tracing::trace;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = LicensedResourcesFetching)]
pub struct LicensedResourcesFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl LicensedResourcesFetching for LicensedResourcesFetchingSqlDbRepository {
    #[tracing::instrument(name = "list_licensed_resources", skip_all)]
    async fn list_licensed_resources(
        &self,
        email: Email,
        tenant: Option<Uuid>,
        roles: Option<Vec<String>>,
        permissioned_roles: Option<PermissionedRoles>,
        related_accounts: Option<RelatedAccounts>,
        was_verified: Option<bool>,
    ) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut sql: String = format!(
            "SELECT * FROM licensed_resources WHERE gu_email = '{}'",
            email.email(),
        );

        if let Some(tenant_id) = tenant {
            sql.push_str(
                format!(
                    " AND (tenant_id = '{}' OR tenant_id IS NULL)",
                    tenant_id.to_string()
                )
                .as_str(),
            );
        }

        if let Some(roles) = roles {
            sql.push_str(
                format!(" AND gr_slug = ANY({})", roles.join(",")).as_str(),
            );
        }

        if let Some(permissioned_roles) = permissioned_roles {
            let statement = permissioned_roles.iter().fold(
                String::new(),
                |acc, (role, permission)| {
                    format!(
                        "{}(gr_slug = '{}' AND gr_perm = {}) OR ",
                        acc,
                        role,
                        permission.to_owned() as i64
                    )
                },
            );

            sql.push_str(format!(" AND ({})", statement).as_str());
        }

        if let Some(was_verified) = was_verified {
            sql.push_str(
                format!(" AND gu_verified = {}", was_verified).as_str(),
            );
        }

        if let Some(related_accounts) = related_accounts {
            if let RelatedAccounts::AllowedAccounts(ids) = related_accounts {
                sql.push_str(
                    format!(
                        " AND acc_id = ANY({})",
                        ids.into_iter()
                            .map(|i| i.to_string())
                            .collect::<Vec<String>>()
                            .join(",")
                    )
                    .as_str(),
                );
            }
        }

        trace!("sql: {:?}", sql);

        let rows = diesel::sql_query(sql)
            .load::<LicensedResourceRow>(conn)
            .map_err(|e| {
                fetching_err(
                    format!("Failed to fetch licensed resources: {e}",),
                )
            })?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let licenses = rows
            .into_iter()
            .map(|record| LicensedResource {
                acc_id: Uuid::parse_str(&record.acc_id).unwrap(),
                tenant_id: record
                    .tenant_id
                    .map(|id| Uuid::parse_str(&id).unwrap())
                    .unwrap_or_else(|| {
                        Uuid::parse_str("00000000-0000-0000-0000-000000000000")
                            .unwrap()
                    }),
                acc_name: record.acc_name,
                sys_acc: record.is_acc_std,
                role: record.gr_slug,
                perm: Permission::from_i32(record.gr_perm),
                verified: record.gu_verified,
            })
            .collect::<Vec<LicensedResource>>();

        Ok(FetchManyResponseKind::Found(licenses))
    }

    #[tracing::instrument(name = "list_tenants_ownership", skip_all)]
    async fn list_tenants_ownership(
        &self,
        email: Email,
        tenant: Option<Uuid>,
    ) -> Result<FetchManyResponseKind<TenantOwnership>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = user_dsl::user
            .into_boxed()
            .inner_join(owner_dsl::owner_on_tenant)
            .filter(user_dsl::email.eq(email.email()))
            .select((owner_dsl::tenant_id, owner_dsl::created));

        if let Some(tenant_id) = tenant {
            query =
                query.filter(owner_dsl::tenant_id.eq(tenant_id.to_string()));
        }

        let rows =
            query.load::<(String, NaiveDateTime)>(conn).map_err(|e| {
                fetching_err(format!("Failed to fetch tenant ownerships: {e}"))
            })?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(
            rows.into_iter()
                .map(|(tenant_id, created)| TenantOwnership {
                    tenant: Uuid::parse_str(&tenant_id).unwrap(),
                    since: created.and_local_timezone(Local).unwrap(),
                })
                .collect(),
        ))
    }
}

#[derive(QueryableByName)]
struct LicensedResourceRow {
    #[diesel(sql_type = Text)]
    acc_id: String,
    #[diesel(sql_type = Text)]
    acc_name: String,
    #[diesel(sql_type = Nullable<Text>)]
    tenant_id: Option<String>,
    #[diesel(sql_type = Bool)]
    is_acc_std: bool,
    #[diesel(sql_type = Text)]
    gr_slug: String,
    #[diesel(sql_type = Integer)]
    gr_perm: i32,
    #[diesel(sql_type = Bool)]
    gu_verified: bool,
}
