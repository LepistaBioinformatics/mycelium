use crate::domain::dtos::token::EmailConfirmationTokenMeta;

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait TokenInvalidation: Interface + Send + Sync {
    /// Get the token and invalidate it on remove it from the store
    ///
    /// This should be used to get the email confirmation token
    async fn get_and_invalidate_email_confirmation_token(
        &self,
        meta: EmailConfirmationTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors>;

    /// Get the token and invalidate it on remove it from the store
    ///
    /// This should be used to get the password change token
    async fn get_and_invalidate_password_change_token(
        &self,
        meta: EmailConfirmationTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors>;
}
