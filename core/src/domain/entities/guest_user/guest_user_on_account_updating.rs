use crate::domain::dtos::guest_role::Permission;

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserOnAccountUpdating: Interface + Send + Sync {
    async fn accept_invitation(
        &self,
        guest_role_name: String,
        account_id: Uuid,
        permission: Permission,
    ) -> Result<UpdatingResponseKind<(String, Uuid, Permission)>, MappedErrors>;
}
