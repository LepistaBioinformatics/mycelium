use crate::domain::{
    dtos::user::UserDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        user: UserDTO,
    ) -> Result<GetOrCreateResponse<UserDTO>, MappedErrors>;

    async fn create(
        &self,
        user: UserDTO,
    ) -> Result<CreateResponse<UserDTO>, MappedErrors>;
}
