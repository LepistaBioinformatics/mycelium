use crate::domain::dtos::error_code::ErrorCode;

use async_trait::async_trait;
use clean_base::{entities::DeletionResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait ErrorCodeDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        error_code: ErrorCode,
    ) -> Result<DeletionResponseKind<ErrorCode>, MappedErrors>;
}
