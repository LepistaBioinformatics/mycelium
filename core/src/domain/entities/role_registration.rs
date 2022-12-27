use crate::domain::dtos::role::RoleDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait RoleRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        role: RoleDTO,
    ) -> Result<GetOrCreateResponseKind<RoleDTO>, MappedErrors>;

    async fn create(
        &self,
        application: RoleDTO,
    ) -> Result<CreateResponseKind<RoleDTO>, MappedErrors>;
}
