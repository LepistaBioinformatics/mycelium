use crate::domain::{
    dtos::account::AccountTypeDTO,
    entities::shared::default_responses::{FetchManyResponse, FetchResponse},
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountTypeFetching: Interface + Send + Sync {
    async fn get(&self, id: String) -> FetchResponse<AccountTypeDTO, Uuid>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponse<AccountTypeDTO, Uuid>, MappedErrors>;
}
