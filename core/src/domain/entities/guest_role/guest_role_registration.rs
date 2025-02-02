use crate::domain::dtos::guest_role::GuestRole;

use async_trait::async_trait;
use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait GuestRoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        guest_role: GuestRole,
    ) -> Result<GetOrCreateResponseKind<GuestRole>, MappedErrors>;
}
