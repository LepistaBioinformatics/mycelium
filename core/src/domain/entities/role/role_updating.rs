use crate::domain::dtos::role::Role;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait RoleUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        role: Role,
    ) -> Result<UpdatingResponseKind<Role>, MappedErrors>;
}
