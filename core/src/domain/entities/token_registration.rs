use crate::domain::dtos::token::TokenDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait TokenRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        token: TokenDTO,
    ) -> Result<CreateResponseKind<TokenDTO>, MappedErrors>;
}
