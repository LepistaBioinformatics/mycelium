use crate::domain::dtos::application::ApplicationDTO;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shaku::Interface;

#[derive(Debug, Serialize, Deserialize)]
pub enum ApplicationFetchResponse {
    Fetched(ApplicationDTO),
    NotFetched,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ApplicationFetchManyResponse {
    Fetched(Vec<ApplicationDTO>),
    NotFetched,
}

#[async_trait]
pub trait ApplicationFetching: Interface + Send + Sync {
    async fn get(&self, id: String) -> ApplicationFetchResponse;
    async fn list(&self, search_term: String) -> ApplicationFetchManyResponse;
}
