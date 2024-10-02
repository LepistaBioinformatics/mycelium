use crate::domain::dtos::tenant::Tenant;

use async_trait::async_trait;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait TenantRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        tenant: Tenant,
    ) -> Result<CreateResponseKind<Tenant>, MappedErrors>;

    async fn register_owner(
        &self,
        tenant_id: Uuid,
        owner_id: Uuid,
    ) -> Result<CreateResponseKind<Tenant>, MappedErrors>;
}

impl Display for dyn TenantRegistration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn TenantRegistration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
