use crate::domain::{
    dtos::role::RoleDTO, entities::shared::default_responses::UpdateResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait ApplicationUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        application: RoleDTO,
    ) -> Result<UpdateResponse<RoleDTO>, MappedErrors>;
}
