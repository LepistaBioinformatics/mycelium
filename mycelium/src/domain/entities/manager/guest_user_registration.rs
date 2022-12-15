use crate::domain::dtos::guest::GuestUserDTO;

use agrobase::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        guest_user: GuestUserDTO,
        account_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<GuestUserDTO>, MappedErrors>;

    async fn create(
        &self,
        guest_user: GuestUserDTO,
    ) -> Result<CreateResponseKind<GuestUserDTO>, MappedErrors>;
}
