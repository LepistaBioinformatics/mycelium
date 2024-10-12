use crate::domain::dtos::email::Email;
use crate::domain::dtos::tenant::TenantMetaKey;

use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait TenantDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;

    async fn delete_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Option<Uuid>,
        owner_email: Option<Email>,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;

    async fn delete_tenant_meta(
        &self,
        tenant_id: Uuid,
        key: TenantMetaKey,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}

impl Display for dyn TenantDeletion {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn TenantDeletion {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
