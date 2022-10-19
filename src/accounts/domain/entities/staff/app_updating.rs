use crate::domain::{
    dtos::application::ApplicationDTO,
    entities::shared::default_responses::UpdateResponse,
};

use async_trait::async_trait;
use shaku::Interface;

#[async_trait]
pub trait ApplicationUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        application: ApplicationDTO,
    ) -> UpdateResponse<ApplicationDTO, ApplicationDTO>;
}
