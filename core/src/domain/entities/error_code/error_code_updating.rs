use crate::domain::dtos::error_code::ErrorCode;

use async_trait::async_trait;
use clean_base::{entities::UpdatingResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait ErrorCodeUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        error_code: ErrorCode,
    ) -> Result<UpdatingResponseKind<ErrorCode>, MappedErrors>;
}
