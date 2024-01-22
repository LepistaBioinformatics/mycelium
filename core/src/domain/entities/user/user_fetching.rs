use crate::domain::dtos::{email::Email, user::User};

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait UserFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Option<Uuid>,
        email: Option<Email>,
        password_hash: Option<String>,
    ) -> Result<FetchResponseKind<User, String>, MappedErrors>;
}
