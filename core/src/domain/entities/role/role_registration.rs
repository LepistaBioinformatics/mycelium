use crate::domain::dtos::role::Role;

use async_trait::async_trait;
use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait RoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        role: Role,
    ) -> Result<GetOrCreateResponseKind<Role>, MappedErrors>;
}
