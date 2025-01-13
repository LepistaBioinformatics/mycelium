use super::TenantOwnerConnection;
use crate::domain::dtos::tenant::{
    Tenant, TenantMeta, TenantMetaKey, TenantStatus,
};

use async_trait::async_trait;
use mycelium_base::entities::CreateResponseKind;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait TenantUpdating: Interface + Send + Sync {
    async fn update_name_and_description(
        &self,
        tenant_id: Uuid,
        tenant: Tenant,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn update_tenant_status(
        &self,
        tenant_id: Uuid,
        status: TenantStatus,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn register_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Uuid,
        guest_by: String,
    ) -> Result<CreateResponseKind<TenantOwnerConnection>, MappedErrors>;

    async fn update_tenant_meta(
        &self,
        tenant_id: Uuid,
        key: TenantMetaKey,
        value: String,
    ) -> Result<UpdatingResponseKind<TenantMeta>, MappedErrors>;
}

impl Display for dyn TenantUpdating {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn TenantUpdating {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
