use crate::domain::dtos::user::UserDTO;

use agrobase::{
    entities::default_response::UpdatingResponseKind,
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait UserUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        user: UserDTO,
    ) -> Result<UpdatingResponseKind<UserDTO>, MappedErrors>;
}
