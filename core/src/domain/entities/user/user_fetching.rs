use crate::domain::dtos::{email::Email, user::User};

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait UserFetching: Interface + Send + Sync {
    async fn get_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors>;

    async fn get_user_by_id(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors>;

    /// Fetches a user by email without redacting secrets
    ///
    /// WARNING: This method should only be used for internal purposes.
    ///
    async fn get_not_redacted_user_by_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors>;
}
