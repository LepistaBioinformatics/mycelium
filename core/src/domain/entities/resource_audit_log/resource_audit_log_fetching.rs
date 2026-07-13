use crate::domain::dtos::resource_audit_log::{
    ResourceAuditLog, ResourceAuditResourceType,
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
pub trait ResourceAuditLogFetching: Interface + Send + Sync {
    async fn list_by_resource(
        &self,
        resource_type: ResourceAuditResourceType,
        resource_id: Uuid,
    ) -> Result<FetchManyResponseKind<ResourceAuditLog>, MappedErrors>;

    async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        page_size: i32,
        skip: i32,
    ) -> Result<FetchManyResponseKind<ResourceAuditLog>, MappedErrors>;
}
