use crate::domain::dtos::role::RoleDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::UpdatingResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait RoleUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        role: RoleDTO,
    ) -> Result<UpdatingResponseKind<RoleDTO>, MappedErrors>;
}
