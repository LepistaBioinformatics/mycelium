use crate::domain::dtos::guest::GuestRoleDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait GuestRoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        user_role: GuestRoleDTO,
    ) -> Result<GetOrCreateResponseKind<GuestRoleDTO>, MappedErrors>;

    async fn create(
        &self,
        user_role: GuestRoleDTO,
    ) -> Result<CreateResponseKind<GuestRoleDTO>, MappedErrors>;
}
