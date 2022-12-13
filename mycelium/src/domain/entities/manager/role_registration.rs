use crate::domain::{
    dtos::role::RoleDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait RoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        application: RoleDTO,
    ) -> Result<GetOrCreateResponse<RoleDTO>, MappedErrors>;

    async fn create(
        &self,
        application: RoleDTO,
    ) -> Result<CreateResponse<RoleDTO>, MappedErrors>;
}
