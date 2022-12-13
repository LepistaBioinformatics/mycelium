use crate::domain::{
    dtos::guest::GuestRoleDTO,
    entities::shared::default_responses::{FetchManyResponse, FetchResponse},
    utils::errors::MappedErrors,
};

use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait UserRoleFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponse<GuestRoleDTO, Uuid>, MappedErrors>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponse<GuestRoleDTO, Uuid>, MappedErrors>;
}
