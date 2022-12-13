use crate::domain::{
    dtos::guest::GuestRoleDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait GuestRoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        user_role: GuestRoleDTO,
    ) -> Result<GetOrCreateResponse<GuestRoleDTO>, MappedErrors>;

    async fn create(
        &self,
        user_role: GuestRoleDTO,
    ) -> Result<CreateResponse<GuestRoleDTO>, MappedErrors>;
}
