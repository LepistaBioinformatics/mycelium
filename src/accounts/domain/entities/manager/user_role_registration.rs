use crate::domain::{
    dtos::guest::UserRoleDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserRoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        user_role: UserRoleDTO,
    ) -> Result<GetOrCreateResponse<UserRoleDTO>, MappedErrors>;

    async fn create(
        &self,
        user_role: UserRoleDTO,
    ) -> Result<CreateResponse<UserRoleDTO>, MappedErrors>;
}
