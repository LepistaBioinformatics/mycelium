use crate::domain::dtos::resource_audit_log::NewResourceAuditLogEvent;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ResourceAuditLogRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        event: NewResourceAuditLogEvent,
    ) -> Result<(), MappedErrors>;
}
