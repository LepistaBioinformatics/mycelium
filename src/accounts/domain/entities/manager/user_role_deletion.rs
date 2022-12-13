use crate::domain::{
    dtos::guest::GuestRoleDTO,
    entities::shared::default_responses::DeleteResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait UserRoleDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        user_role_id: Uuid,
    ) -> Result<DeleteResponse<GuestRoleDTO>, MappedErrors>;
}
