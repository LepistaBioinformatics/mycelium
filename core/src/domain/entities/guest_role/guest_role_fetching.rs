use crate::domain::dtos::guest_role::GuestRole;

use async_trait::async_trait;
use mycelium_base::{
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
    ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors>;
}
