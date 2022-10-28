use crate::domain::{
    dtos::guest::UserRoleDTO,
    entities::shared::default_responses::DeleteResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserRoleDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        user_role: UserRoleDTO,
    ) -> Result<DeleteResponse<UserRoleDTO>, MappedErrors>;
}
