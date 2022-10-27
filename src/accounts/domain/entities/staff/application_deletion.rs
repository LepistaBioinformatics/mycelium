use crate::domain::{
    dtos::application::ApplicationDTO,
    entities::shared::default_responses::DeleteResponse,
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait ApplicationDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        application: ApplicationDTO,
    ) -> Result<DeleteResponse<ApplicationDTO>, MappedErrors>;
}
