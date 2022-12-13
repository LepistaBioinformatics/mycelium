use crate::domain::{
    dtos::role::RoleDTO,
    entities::shared::default_responses::{FetchManyResponse, FetchResponse},
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait RoleFetching: Interface + Send + Sync {
    async fn get(&self, id: String) -> FetchResponse<RoleDTO, Uuid>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponse<RoleDTO, Uuid>, MappedErrors>;
}
