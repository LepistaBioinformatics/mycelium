use crate::domain::dtos::email::Email;
use crate::domain::dtos::tenant::{Tenant, TenantMeta, TenantMetaKey};

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait TenantUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        tenant: Tenant,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn update_name_and_description(
        &self,
        tenant_id: Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn update_tenant_verifying_status(
        &self,
        id: Uuid,
        made_by: String,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn update_tenant_archiving_status(
        &self,
        id: Uuid,
        made_by: String,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn update_tenant_trashing_status(
        &self,
        id: Uuid,
        made_by: String,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn trash_tenant_by_id(
        &self,
        id: Uuid,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn register_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Uuid,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

    async fn remove_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Option<Uuid>,
        owner_email: Option<Email>,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;

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
