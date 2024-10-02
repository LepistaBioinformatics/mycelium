use crate::domain::dtos::{
    related_accounts::RelatedAccounts,
    tenant::{Tenant, TenantMeta, TenantStatus},
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait TenantFetching: Interface + Send + Sync {
    async fn get(
        &self,
        id: Uuid,
        related_accounts: RelatedAccounts,
    ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors>;

    async fn filter(
        &self,
        name: Option<String>,
        owner: Option<Uuid>,
        metadata: Option<TenantMeta>,
        status: Option<TenantStatus>,
        tag_value: Option<String>,
        tag_meta: Option<String>,
    ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors>;
}
