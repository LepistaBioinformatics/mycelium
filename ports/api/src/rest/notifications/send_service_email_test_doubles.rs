//! In-process doubles of the storage ports the send endpoint resolves from
//! the app modules, so its HTTP tests run without a database in every build
//! mode. Only the components the endpoint reaches are overridden; the pool
//! provider is replaced by one that fails the test if anything else asks it
//! for a connection.

use crate::models::active_backend_modules::SqlAppModule;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use myc_core::domain::{
    dtos::{
        email::Email,
        guest_role::Permission,
        message::MessageSendingEvent,
        notification_dispatch::{NewNotificationDispatch, QuotaOutcome},
        profile::{LicensedResource, Profile, TenantOwnership},
        related_accounts::RelatedAccounts,
        security_group::PermissionedRole,
        service::Service,
        tenant::{Tenant, TenantMetaKey, TenantTagFilter},
        token::{PublicConnectionStringInfo, Token, UserAccountScope},
    },
    entities::{
        LicensedResourcesFetching, NotificationDispatchRegistration,
        ProfileFetching, TenantFetching, TokenFetching,
    },
};
use myc_mem_db::repositories::{
    MemDbAppModule, MemDbPoolProvider, MemDbPoolProviderParameters,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// What the storage answers during one send.
#[derive(Clone)]
pub(crate) struct StoredState {
    /// The stored connection string, `None` when expired, revoked or
    /// deleted (the repositories drop those rows).
    pub token: Option<Token>,

    /// The creator profile, `None` when the user is unregistered.
    pub profile: Option<Profile>,

    /// The role the creator holds on the string tenant.
    pub role: String,

    pub tenant_id: Uuid,

    pub quota: QuotaOutcome,
}

// ? ---------------------------------------------------------------------------
// ? Modules
// ? ---------------------------------------------------------------------------

pub(crate) fn sql_module(state: StoredState) -> SqlAppModule {
    let builder = with_unused_pool(SqlAppModule::builder())
        .with_component_override::<dyn TokenFetching>(Box::new(state.clone()))
        .with_component_override::<dyn ProfileFetching>(Box::new(state.clone()))
        .with_component_override::<dyn LicensedResourcesFetching>(Box::new(
            state.clone(),
        ))
        .with_component_override::<dyn TenantFetching>(Box::new(state.clone()))
        .with_component_override::<dyn NotificationDispatchRegistration>(
            Box::new(state),
        );

    with_audit_log_sender(builder).build()
}

pub(crate) fn mem_module(services: Vec<Service>) -> MemDbAppModule {
    MemDbAppModule::builder()
        .with_component_parameters::<MemDbPoolProvider>(
            MemDbPoolProviderParameters {
                services_db: Arc::new(Mutex::new(services)),
                ..Default::default()
            },
        )
        .build()
}

#[cfg(any(feature = "full", feature = "postgres-only"))]
mod pool {
    use myc_diesel::models::config::{DbPool, DbPoolProvider};

    pub(super) struct UnusedPool;

    impl DbPoolProvider for UnusedPool {
        fn get_pool(&self) -> DbPool {
            unreachable!("the send endpoint tests never open a connection")
        }
    }
}

#[cfg(any(feature = "full", feature = "postgres-only"))]
fn with_unused_pool(
    builder: shaku::ModuleBuilder<SqlAppModule>,
) -> shaku::ModuleBuilder<SqlAppModule> {
    builder.with_component_override::<dyn myc_diesel::models::config::DbPoolProvider>(
        Box::new(pool::UnusedPool),
    )
}

#[cfg(any(feature = "full", feature = "postgres-only"))]
fn with_audit_log_sender(
    builder: shaku::ModuleBuilder<SqlAppModule>,
) -> shaku::ModuleBuilder<SqlAppModule> {
    use myc_diesel::repositories::{
        ResourceAuditLogRegistrationSqlDbRepository,
        ResourceAuditLogRegistrationSqlDbRepositoryParameters,
    };

    let (sender, _) = tokio::sync::mpsc::channel(1);

    builder.with_component_parameters::<ResourceAuditLogRegistrationSqlDbRepository>(
        ResourceAuditLogRegistrationSqlDbRepositoryParameters { sender },
    )
}

#[cfg(feature = "standalone")]
mod pool {
    use myc_diesel_sqlite::config::{SqliteDbPool, SqliteDbPoolProvider};

    pub(super) struct UnusedPool;

    impl SqliteDbPoolProvider for UnusedPool {
        fn get_pool(&self) -> SqliteDbPool {
            unreachable!("the send endpoint tests never open a connection")
        }
    }
}

#[cfg(feature = "standalone")]
fn with_unused_pool(
    builder: shaku::ModuleBuilder<SqlAppModule>,
) -> shaku::ModuleBuilder<SqlAppModule> {
    builder.with_component_override::<dyn myc_diesel_sqlite::config::SqliteDbPoolProvider>(
        Box::new(pool::UnusedPool),
    )
}

#[cfg(feature = "standalone")]
fn with_audit_log_sender(
    builder: shaku::ModuleBuilder<SqlAppModule>,
) -> shaku::ModuleBuilder<SqlAppModule> {
    use myc_diesel_sqlite::repositories::resource_audit_log::{
        ResourceAuditLogRegistrationSqlDbRepository,
        ResourceAuditLogRegistrationSqlDbRepositoryParameters,
    };

    let (sender, _) = tokio::sync::mpsc::channel(1);

    builder.with_component_parameters::<ResourceAuditLogRegistrationSqlDbRepository>(
        ResourceAuditLogRegistrationSqlDbRepositoryParameters { sender },
    )
}

// ? ---------------------------------------------------------------------------
// ? Ports
// ? ---------------------------------------------------------------------------

#[async_trait]
impl TokenFetching for StoredState {
    async fn get_connection_string(
        &self,
        _: UserAccountScope,
    ) -> Result<FetchResponseKind<Token, String>, MappedErrors> {
        match self.token.to_owned() {
            Some(token) => Ok(FetchResponseKind::Found(token)),
            None => Ok(FetchResponseKind::NotFound(None)),
        }
    }

    async fn list_connection_strings_by_account_id(
        &self,
        _: Uuid,
    ) -> Result<FetchManyResponseKind<PublicConnectionStringInfo>, MappedErrors>
    {
        unreachable!()
    }
}

#[async_trait]
impl ProfileFetching for StoredState {
    async fn get_from_email(
        &self,
        _: Email,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        match self.profile.to_owned() {
            Some(profile) => Ok(FetchResponseKind::Found(profile)),
            None => Ok(FetchResponseKind::NotFound(None)),
        }
    }

    async fn get_from_token(
        &self,
        _: String,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        unreachable!()
    }
}

#[async_trait]
impl LicensedResourcesFetching for StoredState {
    async fn list_licensed_resources(
        &self,
        _: Email,
        _: Option<Uuid>,
        _: Option<Vec<PermissionedRole>>,
        _: Option<RelatedAccounts>,
        _: Option<bool>,
    ) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors> {
        Ok(FetchManyResponseKind::Found(vec![LicensedResource {
            acc_id: Uuid::new_v4(),
            tenant_id: self.tenant_id,
            role_id: Uuid::new_v4(),
            acc_name: "Account".to_string(),
            sys_acc: false,
            role: self.role.to_owned(),
            perm: Permission::Write,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }]))
    }

    async fn list_tenants_ownership(
        &self,
        _: Email,
        _: Option<Uuid>,
    ) -> Result<FetchManyResponseKind<TenantOwnership>, MappedErrors> {
        Ok(FetchManyResponseKind::NotFound)
    }
}

#[async_trait]
impl TenantFetching for StoredState {
    async fn get_tenant_owned_by_me(
        &self,
        _: Uuid,
        _: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        unreachable!()
    }

    async fn get_tenant_public_by_id(
        &self,
        _: Uuid,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        Ok(FetchResponseKind::NotFound(None))
    }

    async fn get_tenants_by_manager_account(
        &self,
        _: Uuid,
        _: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
        unreachable!()
    }

    async fn filter_tenants_as_manager(
        &self,
        _: Option<String>,
        _: Option<Uuid>,
        _: Option<(TenantMetaKey, String)>,
        _: TenantTagFilter,
        _: Option<i32>,
        _: Option<i32>,
    ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
        unreachable!()
    }
}

#[async_trait]
impl NotificationDispatchRegistration for StoredState {
    async fn register_and_enqueue(
        &self,
        _: &str,
        _: u32,
        rows: Vec<NewNotificationDispatch>,
        _: Vec<MessageSendingEvent>,
    ) -> Result<QuotaOutcome, MappedErrors> {
        match self.quota {
            QuotaOutcome::Accepted(_) => Ok(QuotaOutcome::Accepted(
                rows.iter().map(|row| row.id).collect(),
            )),
            _ => Ok(self.quota.to_owned()),
        }
    }

    async fn purge_older_than(
        &self,
        _: DateTime<Utc>,
    ) -> Result<u64, MappedErrors> {
        unreachable!()
    }
}
