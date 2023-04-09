use crate::domain::dtos::guest::GuestRole;

use async_trait::async_trait;
use clean_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestRoleFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors>;
    async fn list(
        &self,
        name: Option<String>,
        role_id: Option<Uuid>,
    ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors>;
}
