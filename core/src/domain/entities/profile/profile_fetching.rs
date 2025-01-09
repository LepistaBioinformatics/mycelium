use crate::domain::dtos::{email::Email, profile::Profile};

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[async_trait]
pub trait ProfileFetching: Interface + Send + Sync {
    async fn get_from_email(
        &self,
        email: Email,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors>;

    async fn get_from_token(
        &self,
        token: String,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors>;
}
