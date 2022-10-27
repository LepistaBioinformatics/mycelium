use crate::domain::{
    dtos::user::UserDTO, entities::shared::default_responses::UpdateResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        user: UserDTO,
    ) -> Result<UpdateResponse<UserDTO>, MappedErrors>;
}
