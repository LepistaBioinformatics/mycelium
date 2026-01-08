use crate::{
    models::{config::DbPoolProvider, licensed_resource::LicensedResourceRow},
    schema::{
        owner_on_tenant::dsl as owner_dsl, tenant::dsl as tenant_dsl,
        user::dsl as user_dsl,
    },
};

use async_trait::async_trait;
use chrono::{Local, NaiveDateTime};
use diesel::{prelude::*, RunQueryDsl};
use myc_core::domain::{
    dtos::{
        email::Email,
        guest_role::Permission,
        native_error_codes::NativeErrorCodes,
        profile::{LicensedResource, TenantOwnership},
        related_accounts::RelatedAccounts,
        security_group::PermissionedRole,
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
        roles: Option<Vec<PermissionedRole>>,
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

        tracing::debug!("Roles in List Licensed Resources: {:?}", roles);

        if let Some(roles) = roles {
            let statement = roles
                .iter()
                .fold(String::new(), |acc, role| {
                    format!(
                        "{}(gr_slug = '{}' AND gr_perm >= {}) OR ",
                        acc,
                        role.name,
                        role.permission.to_owned().clone().unwrap_or_default()
                            as i64
                    )
                })
                .trim_end_matches(" OR ")
                .to_string();

            sql.push_str(format!(" AND ({})", statement).as_str());
        }

        tracing::debug!("SQL Query: {}", sql);

        if let Some(was_verified) = was_verified {
            sql.push_str(
                format!(" AND gu_verified = {}", was_verified).as_str(),
            );
        }

        if let Some(related_accounts) = related_accounts {
            match related_accounts {
                RelatedAccounts::AllowedAccounts(ids) => {
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
                RelatedAccounts::HasTenantWidePrivileges(tenant_id) => {
                    sql.push_str(
                        format!(" AND tenant_id = '{}'", tenant_id.to_string())
                            .as_str(),
                    );
                }
                _ => (),
            }
        }

        trace!(
            "sql: {s}",
            s = sql.replace(&email.email(), &email.redacted_email())
        );

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
                acc_id: record.acc_id,
                role_id: record.gr_id,
                tenant_id: record.tenant_id.unwrap_or_else(|| Uuid::nil()),
                acc_name: record.acc_name,
                sys_acc: record.is_acc_std,
                role: record.gr_slug,
                perm: Permission::from_i32(record.gr_perm),
                verified: record.gu_verified,
                permit_flags: record.permit_flags,
                deny_flags: record.deny_flags,
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
            .inner_join(
                tenant_dsl::tenant.on(owner_dsl::tenant_id.eq(tenant_dsl::id)),
            )
            .filter(user_dsl::email.eq(email.email()))
            .select((
                owner_dsl::tenant_id,
                owner_dsl::created,
                tenant_dsl::name,
            ));

        if let Some(tenant_id) = tenant {
            query = query.filter(owner_dsl::tenant_id.eq(tenant_id));
        }

        let rows =
            query
                .load::<(Uuid, NaiveDateTime, String)>(conn)
                .map_err(|e| {
                    fetching_err(format!(
                        "Failed to fetch tenant ownerships: {e}"
                    ))
                })?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(
            rows.into_iter()
                .map(|(tenant_id, created, tenant_name)| TenantOwnership {
                    id: tenant_id,
                    since: created.and_local_timezone(Local).unwrap(),
                    name: tenant_name,
                })
                .collect(),
        ))
    }
}
