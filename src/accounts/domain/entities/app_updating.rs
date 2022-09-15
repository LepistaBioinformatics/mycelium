use crate::domain::dtos::application::ApplicationDTO;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shaku::Interface;

#[derive(Debug, Serialize, Deserialize)]
pub enum ApplicationUpdateResponse {
    Updated(ApplicationDTO),
    NotUpdated(ApplicationDTO),
}

#[async_trait]
pub trait ApplicationUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        application: ApplicationDTO,
    ) -> ApplicationUpdateResponse;
}
