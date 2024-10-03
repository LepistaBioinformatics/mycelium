use crate::domain::dtos::{
    email::Email, profile::LicensedResources, related_accounts::RelatedAccounts,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait LicensedResourcesFetching: Interface + Send + Sync {
    async fn list(
        &self,
        email: Email,
        related_accounts: Option<RelatedAccounts>,
    ) -> Result<FetchManyResponseKind<LicensedResources>, MappedErrors>;
}
