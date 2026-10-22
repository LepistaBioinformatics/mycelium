use crate::domain::dtos::{
    email::Email, guest_user_on_account::GuestUserOnAccount,
};

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait GuestUserOnAccountUpdating: Interface + Send + Sync {
    /// Mark the caller's own pending invitation as verified
    ///
    /// Only the unverified guest of `account_id` whose email is one of
    /// `guest_emails` and whose role is `guest_role_id` is touched; its role
    /// is not changed. Returns the id of the verified guest user.
    async fn accept_invitation(
        &self,
        guest_emails: Vec<Email>,
        account_id: Uuid,
        guest_role_id: Uuid,
    ) -> Result<UpdatingResponseKind<Uuid>, MappedErrors>;

    async fn update(
        &self,
        guest_user_on_account: GuestUserOnAccount,
    ) -> Result<UpdatingResponseKind<GuestUserOnAccount>, MappedErrors>;
}
