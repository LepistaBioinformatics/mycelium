use crate::domain::dtos::account::AccountTypeDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountTypeFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: String,
    ) -> Result<FetchResponseKind<AccountTypeDTO, Uuid>, MappedErrors>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponseKind<AccountTypeDTO>, MappedErrors>;
}
