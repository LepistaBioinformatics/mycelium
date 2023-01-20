use crate::domain::dtos::{email::Email, guest::GuestUser};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchManyResponseKind,
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestUserFetching: Interface + Send + Sync {
    async fn list(
        &self,
        account_id: Option<Uuid>,
        email: Option<Email>,
    ) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors>;
}
