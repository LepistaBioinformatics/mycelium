use crate::domain::dtos::application::ApplicationDTO;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shaku::Interface;

#[derive(Debug, Serialize, Deserialize)]
pub enum ApplicationGetOrCreateResponse {
    Created(ApplicationDTO),
    NotCreated(ApplicationDTO),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ApplicationCreateResponse {
    Created(ApplicationDTO),
    NotCreated(ApplicationDTO),
}

#[async_trait]
pub trait ApplicationRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        application: ApplicationDTO,
    ) -> ApplicationGetOrCreateResponse;

    async fn create(
        &self,
        application: ApplicationDTO,
    ) -> ApplicationCreateResponse;
}
