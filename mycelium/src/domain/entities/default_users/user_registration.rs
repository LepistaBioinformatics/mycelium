use crate::domain::dtos::user::UserDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait UserRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        user: UserDTO,
    ) -> Result<GetOrCreateResponseKind<UserDTO>, MappedErrors>;

    async fn create(
        &self,
        user: UserDTO,
    ) -> Result<CreateResponseKind<UserDTO>, MappedErrors>;
}
