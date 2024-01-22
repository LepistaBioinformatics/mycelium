use crate::domain::dtos::error_code::ErrorCode;

use async_trait::async_trait;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait ErrorCodeRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        error_code: ErrorCode,
    ) -> Result<CreateResponseKind<ErrorCode>, MappedErrors>;
}
