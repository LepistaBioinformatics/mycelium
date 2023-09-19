use crate::domain::dtos::{email::Email, user::User};

use async_trait::async_trait;
use clean_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait UserFetching: Interface + Send + Sync {
    async fn get(
        &self,
        email: Email,
        password_hash: Option<String>,
    ) -> Result<FetchResponseKind<User, Email>, MappedErrors>;
}
