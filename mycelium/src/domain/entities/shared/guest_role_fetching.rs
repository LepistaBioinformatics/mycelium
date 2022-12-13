use crate::domain::dtos::guest::GuestRoleDTO;

use agrobase::{
    entities::default_response::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestRoleFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<GuestRoleDTO, Uuid>, MappedErrors>;
    async fn list(
        &self,
        search_term: String,
    ) -> Result<FetchManyResponseKind<GuestRoleDTO>, MappedErrors>;
}
