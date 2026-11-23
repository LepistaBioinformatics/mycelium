use crate::domain::dtos::error_code::ErrorCode;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ErrorCodeRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        error_code: ErrorCode,
    ) -> Result<CreateResponseKind<ErrorCode>, MappedErrors>;
}
