use crate::domain::dtos::tenant::Tenant;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};

#[async_trait]
pub trait TenantUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        tag: Tenant,
    ) -> Result<UpdatingResponseKind<Tenant>, MappedErrors>;
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
