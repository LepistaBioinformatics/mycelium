use crate::domain::{
    dtos::role::RoleDTO, entities::shared::default_responses::DeleteResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait RoleDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        application: RoleDTO,
    ) -> Result<DeleteResponse<RoleDTO>, MappedErrors>;
}
