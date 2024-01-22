use crate::domain::dtos::guest::GuestUser;

use async_trait::async_trait;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserFetching: Interface + Send + Sync {
    async fn list(
        &self,
        account_id: Uuid,
    ) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors>;
}
