use crate::domain::dtos::user::User;

use async_trait::async_trait;
use mycelium_base::{
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait UserRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        user: User,
    ) -> Result<GetOrCreateResponseKind<User>, MappedErrors>;

    async fn create(
        &self,
        user: User,
    ) -> Result<CreateResponseKind<User>, MappedErrors>;
}
