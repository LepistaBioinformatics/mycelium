use crate::domain::dtos::guest_role::GuestRole;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestRoleUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        user_role: GuestRole,
    ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors>;

    async fn insert_role_children(
        &self,
        role_id: Uuid,
        child_id: Uuid,
    ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors>;

    async fn remove_role_children(
        &self,
        role_id: Uuid,
        child_id: Uuid,
    ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors>;
}
