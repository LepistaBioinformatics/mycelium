use crate::domain::{
    dtos::application::ApplicationDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait ApplicationRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        application: ApplicationDTO,
    ) -> Result<GetOrCreateResponse<ApplicationDTO>, MappedErrors>;

    async fn create(
        &self,
        application: ApplicationDTO,
    ) -> Result<CreateResponse<ApplicationDTO>, MappedErrors>;
}
