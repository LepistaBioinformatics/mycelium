use crate::domain::dtos::token::Token;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait TokenDeregistration: Interface + Send + Sync {
    /// Get then remove token from the data source.
    ///
    /// Get the record form the data-source and if exists, remove it.
    async fn get_then_delete(
        &self,
        token: Token,
    ) -> Result<FetchResponseKind<Token, Uuid>, MappedErrors>;
}
