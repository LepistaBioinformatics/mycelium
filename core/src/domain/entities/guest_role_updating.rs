use crate::domain::dtos::guest::GuestRole;

use async_trait::async_trait;
use clean_base::{entities::UpdatingResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait GuestRoleUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        user_role: GuestRole,
    ) -> Result<UpdatingResponseKind<GuestRole>, MappedErrors>;
}
