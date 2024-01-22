use crate::domain::dtos::role::Role;

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait RoleFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<Role, Uuid>, MappedErrors>;
    async fn list(
        &self,
        name: Option<String>,
    ) -> Result<FetchManyResponseKind<Role>, MappedErrors>;
}
