use crate::{
    config::SqliteDbPoolProvider,
    models::licensed_resource::LicensedResourceRow,
    schema::{owner_on_tenant, tenant, user},
    types::{string_array_from_text, uuid_from_text, uuid_to_text},
};

use async_trait::async_trait;
use chrono::Local;
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

/// Defensive quoting for string values that end up interpolated into the
/// dynamically-built query below (diesel's raw `sql_query` bind chain is
/// static, so a runtime-variable number/type of filters can't all go through
/// `.bind()`). Numeric/bool/UUID values are interpolated via `Display`, which
/// can't emit quotes; only free-form strings (email, role slug) go through
/// this escape.
fn sql_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[async_trait]
impl LicensedResourcesFetching for LicensedResourcesFetchingSqlDbRepository {
    #[tracing::instrument(name = "list_licensed_resources", skip_all)]
    async fn list_licensed_resources(
        &self,
        email: Email,
        tenant_id_filter: Option<Uuid>,
        roles: Option<Vec<PermissionedRole>>,
        related_accounts: Option<RelatedAccounts>,
        was_verified: Option<bool>,
    ) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut sql: String = format!(
            "SELECT * FROM licensed_resources WHERE gu_email = {}",
            sql_quote(&email.email()),
        );

        if let Some(tenant_id) = tenant_id_filter {
            sql.push_str(
                format!(
                    " AND (tenant_id = {} OR tenant_id IS NULL)",
                    sql_quote(&uuid_to_text(&tenant_id))
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
                        "{}(gr_slug = {} AND gr_perm >= {}) OR ",
                        acc,
                        sql_quote(&role.name),
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
                format!(" AND gu_verified = {}", was_verified as i32).as_str(),
            );
        }

        if let Some(related_accounts) = related_accounts {
            match related_accounts {
                RelatedAccounts::AllowedAccounts(ids) => {
                    sql.push_str(
                        format!(
                            " AND acc_id IN ({})",
                            ids.into_iter()
                                .map(|i| sql_quote(&uuid_to_text(&i)))
                                .collect::<Vec<String>>()
                                .join(",")
                        )
                        .as_str(),
                    );
                }
                RelatedAccounts::HasTenantWidePrivileges(tenant_id) => {
                    sql.push_str(
                        format!(
                            " AND tenant_id = {}",
                            sql_quote(&uuid_to_text(&tenant_id))
                        )
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
                acc_id: uuid_from_text(&record.acc_id).unwrap(),
                role_id: uuid_from_text(&record.gr_id).unwrap(),
                tenant_id: record
                    .tenant_id
                    .map(|t| uuid_from_text(&t).unwrap())
                    .unwrap_or_else(Uuid::nil),
                acc_name: record.acc_name,
                sys_acc: record.is_acc_std,
                role: record.gr_slug,
                perm: Permission::from_i32(record.gr_perm),
                verified: record.gu_verified,
                permit_flags: record
                    .permit_flags
                    .map(|f| string_array_from_text(&f).unwrap()),
                deny_flags: record
                    .deny_flags
                    .map(|f| string_array_from_text(&f).unwrap()),
            })
            .collect::<Vec<LicensedResource>>();

        Ok(FetchManyResponseKind::Found(licenses))
    }

    #[tracing::instrument(name = "list_tenants_ownership", skip_all)]
    async fn list_tenants_ownership(
        &self,
        email: Email,
        tenant_id_filter: Option<Uuid>,
    ) -> Result<FetchManyResponseKind<TenantOwnership>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = user::table
            .into_boxed()
            .inner_join(owner_on_tenant::table)
            .inner_join(
                tenant::table.on(owner_on_tenant::tenant_id.eq(tenant::id)),
            )
            .filter(user::email.eq(email.email()))
            .select((
                owner_on_tenant::tenant_id,
                owner_on_tenant::created,
                tenant::name,
            ));

        if let Some(tenant_id) = tenant_id_filter {
            query = query.filter(
                owner_on_tenant::tenant_id.eq(uuid_to_text(&tenant_id)),
            );
        }

        let rows =
            query.load::<(String, String, String)>(conn).map_err(|e| {
                fetching_err(format!("Failed to fetch tenant ownerships: {e}"))
            })?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(
            rows.into_iter()
                .map(|(tenant_id, created, tenant_name)| TenantOwnership {
                    id: uuid_from_text(&tenant_id).unwrap(),
                    since: crate::types::naive_timestamp_from_text(&created)
                        .unwrap()
                        .and_local_timezone(Local)
                        .unwrap(),
                    name: tenant_name,
                })
                .collect(),
        ))
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::{
            guest_role::GuestRoleRegistrationSqlDbRepository,
            guest_user::GuestUserRegistrationSqlDbRepository,
            tenant::TenantRegistrationSqlDbRepository,
        },
        schema::account,
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::{
            guest_role::{GuestRole, Permission},
            guest_user::GuestUser,
            profile::Owner,
            security_group::PermissionedRole,
            tenant::Tenant,
        },
        entities::{
            GuestRoleRegistration, GuestUserRegistration, TenantRegistration,
        },
    };
    use mycelium_base::dtos::{Children, Parent};

    #[tokio::test]
    async fn list_licensed_resources_and_tenant_ownership_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();

        let tenant_registration = TenantRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let role_registration = GuestRoleRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let guest_registration = GuestUserRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = LicensedResourcesFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let owner_id = Uuid::new_v4();
        let owner_email = "owner@acme.test";

        // Seed the owner user row (owner_on_tenant.owner_id FK -> user(id))
        {
            let conn = &mut db.provider.get_pool().get().unwrap();
            diesel::insert_into(user::table)
                .values((
                    user::id.eq(uuid_to_text(&owner_id)),
                    user::username.eq("owner"),
                    user::email.eq(owner_email),
                    user::first_name.eq("Own"),
                    user::last_name.eq("Er"),
                    user::is_active.eq(true),
                    user::created.eq(crate::types::naive_timestamp_to_text(
                        &chrono::Utc::now().naive_utc(),
                    )),
                    user::is_principal.eq(true),
                ))
                .execute(conn)
                .unwrap();
        }

        // Create tenant (also creates owner_on_tenant for tenant ownership)
        let tenant = Tenant::new_with_owners(
            "Acme Tenant".into(),
            None,
            Children::Records(vec![Owner {
                id: owner_id,
                email: owner_email.into(),
                first_name: Some("Own".into()),
                last_name: Some("Er".into()),
                username: Some("owner".into()),
                is_principal: true,
            }]),
        );
        let created_tenant =
            match tenant_registration.create(tenant, "self".into()).await? {
                mycelium_base::entities::CreateResponseKind::Created(t) => t,
                mycelium_base::entities::CreateResponseKind::NotCreated(..) => {
                    panic!("expected the tenant to be created")
                }
            };
        let tenant_id = created_tenant.id.expect("tenant must have an id");

        // Seed an account under that tenant (licensed_resources view needs it)
        let account_id = Uuid::new_v4();
        {
            let conn = &mut db.provider.get_pool().get().unwrap();
            diesel::insert_into(account::table)
                .values((
                    account::id.eq(uuid_to_text(&account_id)),
                    account::name.eq("Guest Account"),
                    account::slug.eq("guest-account"),
                    account::tenant_id.eq(uuid_to_text(&tenant_id)),
                    account::created.eq(crate::types::naive_timestamp_to_text(
                        &chrono::Utc::now().naive_utc(),
                    )),
                ))
                .execute(conn)
                .unwrap();
        }

        // Create a guest role and a guest user granted access to the account
        let role = GuestRole::new(
            None,
            "Collaborator".into(),
            None,
            Permission::Write,
            None,
            false,
        );
        let role = match role_registration.get_or_create(role).await? {
            mycelium_base::entities::GetOrCreateResponseKind::Created(r) => r,
            mycelium_base::entities::GetOrCreateResponseKind::NotCreated(
                ..,
            ) => panic!("expected the role to be created"),
        };
        let role_id = role.id.expect("role must have an id");

        let guest_email = "guest@acme.test";
        let guest_dto = GuestUser::new_unverified(
            Email::from_string(guest_email.into())?,
            Parent::Id(role_id),
            None,
        );
        guest_registration
            .get_or_create(guest_dto, account_id)
            .await?;

        // list_licensed_resources finds the guest's access to the account
        let licenses = match fetching
            .list_licensed_resources(
                Email::from_string(guest_email.into())?,
                Some(tenant_id),
                // `GuestRole::new` slugifies the name ("Collaborator" ->
                // "collaborator"); `gr_slug` in the view stores the slug, so
                // the filter must match against it, not the display name.
                Some(vec![PermissionedRole {
                    name: "collaborator".into(),
                    permission: Some(Permission::Write),
                }]),
                None,
                None,
            )
            .await?
        {
            FetchManyResponseKind::Found(records) => records,
            _ => panic!("expected to find licensed resources"),
        };
        assert_eq!(licenses.len(), 1);
        assert_eq!(licenses[0].acc_id, account_id);
        assert_eq!(licenses[0].role_id, role_id);

        // list_tenants_ownership finds the owner's tenant
        let ownerships = match fetching
            .list_tenants_ownership(
                Email::from_string(owner_email.into())?,
                None,
            )
            .await?
        {
            FetchManyResponseKind::Found(records) => records,
            _ => panic!("expected to find tenant ownerships"),
        };
        assert_eq!(ownerships.len(), 1);
        assert_eq!(ownerships[0].id, tenant_id);
        assert_eq!(ownerships[0].name, "Acme Tenant");

        Ok(())
    }
}
