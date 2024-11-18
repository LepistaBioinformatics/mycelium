use crate::domain::dtos::{
    email::Email, profile::LicensedResource, related_accounts::RelatedAccounts,
    route_type::PermissionedRoles,
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
        roles: Option<Vec<String>>,
        permissioned_roles: Option<PermissionedRoles>,
        related_accounts: Option<RelatedAccounts>,
    ) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors>;
}
