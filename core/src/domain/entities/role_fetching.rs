use crate::domain::dtos::role::RoleDTO;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait RoleFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<RoleDTO, Uuid>, MappedErrors>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponseKind<RoleDTO>, MappedErrors>;
}
