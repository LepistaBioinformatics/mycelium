use crate::domain::dtos::user::{PasswordHash, User};

use async_trait::async_trait;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait UserUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        user: User,
    ) -> Result<UpdatingResponseKind<User>, MappedErrors>;

    async fn update_password(
        &self,
        user_id: Uuid,
        new_password: PasswordHash,
    ) -> Result<UpdatingResponseKind<bool>, MappedErrors>;
}
