use crate::domain::{
    dtos::application::ApplicationDTO,
    entities::shared::default_responses::{FetchManyResponse, FetchResponse},
};

use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait ApplicationFetching: Interface + Send + Sync {
    async fn get(&self, id: String) -> FetchResponse<ApplicationDTO, Uuid>;
    async fn list(
        &self,
        search_term: String,
    ) -> FetchManyResponse<ApplicationDTO, Uuid>;
}
