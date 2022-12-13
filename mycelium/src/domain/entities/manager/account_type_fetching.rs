use crate::domain::dtos::account::AccountTypeDTO;

use agrobase::{
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountTypeFetching: Interface + Send + Sync {
    async fn get(&self, id: String) -> FetchResponseKind<AccountTypeDTO, Uuid>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponseKind<AccountTypeDTO>, MappedErrors>;
}
