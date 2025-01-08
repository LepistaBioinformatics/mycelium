use crate::domain::dtos::{
    email::Email,
    profile::{LicensedResource, TenantOwnership},
    related_accounts::RelatedAccounts,
    route_type::PermissionedRoles,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait LicensedResourcesFetching: Interface + Send + Sync {
    async fn list_licensed_resources(
        &self,
        email: Email,
        tenant: Option<Uuid>,
        roles: Option<Vec<String>>,
        permissioned_roles: Option<PermissionedRoles>,
        related_accounts: Option<RelatedAccounts>,
        was_verified: Option<bool>,
    ) -> Result<FetchManyResponseKind<LicensedResource>, MappedErrors>;

    async fn list_tenants_ownership(
        &self,
        email: Email,
        tenant: Option<Uuid>,
    ) -> Result<FetchManyResponseKind<TenantOwnership>, MappedErrors>;
}
