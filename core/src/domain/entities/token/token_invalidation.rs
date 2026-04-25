use crate::domain::dtos::{email::Email, token::EmailConfirmationTokenMeta};

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

    /// Phase 1 of magic link flow: consume the display token, return the code
    ///
    /// Fetches the record by (email, token). If found and not expired,
    /// invalidates the token (sets it to None) and returns the 6-digit code.
    /// If not found or expired, returns `NotFound`.
    async fn get_code_and_invalidate_display_token(
        &self,
        email: &Email,
        token: &str,
    ) -> Result<FetchResponseKind<String, String>, MappedErrors>;

    /// Phase 2 of magic link flow: consume the code, delete the record
    ///
    /// Fetches the record by (email, code) where the display token has already
    /// been consumed (token IS NULL). If found and not expired, deletes the
    /// record and returns `Found(())`. If not found, returns `NotFound`.
    async fn get_and_invalidate_magic_link_code(
        &self,
        email: &Email,
        code: &str,
    ) -> Result<FetchResponseKind<(), String>, MappedErrors>;
}
