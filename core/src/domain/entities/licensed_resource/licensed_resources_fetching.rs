use crate::domain::dtos::{
    email::Email,
    profile::{LicensedResource, TenantOwnership},
    related_accounts::RelatedAccounts,
    security_group::PermissionedRole,
};

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait LicensedResourcesFetching: Interface + Send + Sync {
    async fn list_licensed_resources(
        &self,
        email: Email,
        tenant: Option<Uuid>,
        roles: Option<Vec<PermissionedRole>>,
        related_accounts: Option<RelatedAccounts>,
        was_verified: Option<bool>,
    ) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors>;

    async fn list_tenants_ownership(
        &self,
        email: Email,
        tenant: Option<Uuid>,
    ) -> Result<FetchManyResponseKind<TenantOwnership>, MappedErrors>;
}
