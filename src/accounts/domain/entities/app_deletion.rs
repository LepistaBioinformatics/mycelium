use crate::domain::dtos::application::ApplicationDTO;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shaku::Interface;

#[derive(Debug, Serialize, Deserialize)]
pub enum ApplicationDeleteResponse {
    Deleted(String),
    NotDeleted(String),
}

#[async_trait]
pub trait ApplicationDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        application: ApplicationDTO,
    ) -> ApplicationDeleteResponse;
}
