use crate::domain::dtos::guest::GuestUserDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        guest_user: GuestUserDTO,
        account_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<GuestUserDTO>, MappedErrors>;
}
