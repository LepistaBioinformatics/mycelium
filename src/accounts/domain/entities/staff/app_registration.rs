use crate::domain::{
    dtos::application::ApplicationDTO,
    entities::shared::default_responses::{
        CreateResponse, GetOrCreateResponse,
    },
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait ApplicationRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        application: ApplicationDTO,
    ) -> GetOrCreateResponse<ApplicationDTO, ApplicationDTO>;

    async fn create(
        &self,
        application: ApplicationDTO,
    ) -> CreateResponse<ApplicationDTO, ApplicationDTO>;
}
