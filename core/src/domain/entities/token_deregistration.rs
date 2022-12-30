use crate::domain::dtos::{role::RoleDTO, token::TokenDTO};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::DeletionResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait TokenDeregistration: Interface + Send + Sync {
    /// Get then remove token from the data source.
    ///
    /// Get the record form the data-source and if exists, remove it.
    async fn get_then_delete(
        &self,
        token: TokenDTO,
        requesting_service: String,
    ) -> Result<DeletionResponseKind<RoleDTO>, MappedErrors>;
}
