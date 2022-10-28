use crate::domain::{
    dtos::guest::UserRoleDTO,
    entities::shared::default_responses::UpdateResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserRoleUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        user_role: UserRoleDTO,
    ) -> Result<UpdateResponse<UserRoleDTO>, MappedErrors>;
}
