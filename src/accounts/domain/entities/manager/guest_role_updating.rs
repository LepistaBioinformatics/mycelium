use crate::domain::{
    dtos::guest::GuestRoleDTO,
    entities::shared::default_responses::UpdateResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait GuestRoleUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        user_role: GuestRoleDTO,
    ) -> Result<UpdateResponse<GuestRoleDTO>, MappedErrors>;
}
