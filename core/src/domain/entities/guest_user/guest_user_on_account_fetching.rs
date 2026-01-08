use crate::domain::dtos::guest_user_on_account::GuestUserOnAccount;

use async_trait::async_trait;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserOnAccountFetching: Interface + Send + Sync {
    async fn list_by_guest_role_id(
        &self,
        guest_role_id: Uuid,
        account_id: Uuid,
    ) -> Result<FetchManyResponseKind<GuestUserOnAccount>, MappedErrors>;
}
