use crate::domain::dtos::role::RoleDTO;

use agrobase::{
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait RoleFetching: Interface + Send + Sync {
    async fn get(&self, id: String) -> FetchResponseKind<RoleDTO, Uuid>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponseKind<RoleDTO>, MappedErrors>;
}
