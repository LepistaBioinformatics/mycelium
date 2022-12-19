use crate::domain::dtos::{email::EmailDTO, profile::ProfileDTO};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait ProfileFetching: Interface + Send + Sync {
    async fn get(
        &self,
        email: EmailDTO,
    ) -> Result<FetchResponseKind<ProfileDTO, EmailDTO>, MappedErrors>;
}
