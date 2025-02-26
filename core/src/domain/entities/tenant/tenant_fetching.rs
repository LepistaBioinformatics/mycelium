use crate::domain::dtos::tenant::{Tenant, TenantMetaKey};

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait TenantFetching: Interface + Send + Sync {
    /// Get tenant owned by me
    ///
    /// This use-case should be used by tenant-owners only
    async fn get_tenant_owned_by_me(
        &self,
        id: Uuid,
        owners_ids: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors>;

    /// Get tenant public by id
    ///
    /// This use-case should be used by non-privileged users
    async fn get_tenant_public_by_id(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors>;

    /// Get tenants with we are tenant manager
    ///
    /// This use-case should ve used by tenant-managers only
    async fn get_tenants_by_manager_account(
        &self,
        id: Uuid,
        manager_ids: Vec<Uuid>,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors>;

    /// Get tenants with we are manager
    ///
    /// This use-case should ve used by managers only
    async fn filter_tenants_as_manager(
        &self,
        name: Option<String>,
        owner: Option<Uuid>,
        metadata: Option<(TenantMetaKey, String)>,
        tag: Option<(String, String)>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors>;
}
