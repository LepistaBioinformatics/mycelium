use crate::domain::dtos::guest::GuestRoleDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait GuestRoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        guest_role: GuestRoleDTO,
    ) -> Result<GetOrCreateResponseKind<GuestRoleDTO>, MappedErrors>;
}
