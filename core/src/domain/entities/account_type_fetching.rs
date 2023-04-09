use crate::domain::dtos::account::AccountType;

use async_trait::async_trait;
use clean_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait AccountTypeFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<AccountType, Uuid>, MappedErrors>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponseKind<AccountType>, MappedErrors>;
}
