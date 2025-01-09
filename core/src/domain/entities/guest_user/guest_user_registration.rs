use crate::domain::dtos::guest_user::GuestUser;

use async_trait::async_trait;
use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        guest_user: GuestUser,
        account_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors>;
}
