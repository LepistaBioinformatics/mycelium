use crate::domain::dtos::guest::GuestUser;

use async_trait::async_trait;
use clean_base::{entities::UpdatingResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserOnAccountUpdating: Interface + Send + Sync {
    async fn update(
        &self,
        account_id: Uuid,
        old_guest_user_id: Uuid,
        new_guest_user_id: Uuid,
    ) -> Result<UpdatingResponseKind<GuestUser>, MappedErrors>;
}
