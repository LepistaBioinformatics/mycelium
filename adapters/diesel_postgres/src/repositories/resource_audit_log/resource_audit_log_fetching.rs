use super::resource_audit_log_db_encoding::{
    event_kind_from_db_str, resource_type_from_db_str,
};
use crate::{
    models::{
        config::DbPoolProvider,
        resource_audit_log::ResourceAuditLog as ResourceAuditLogModel,
    },
    schema::resource_audit_log as resource_audit_log_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::resource_audit_log::{ResourceAuditLog, ResourceAuditResourceType},
    entities::ResourceAuditLogFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = ResourceAuditLogFetching)]
pub struct ResourceAuditLogFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ResourceAuditLogFetching for ResourceAuditLogFetchingSqlDbRepository {
    #[tracing::instrument(
        name = "list_resource_audit_log_by_resource",
        skip_all
    )]
    async fn list_by_resource(
        &self,
        // `resource_id` alone (a UUID) already narrows to the right rows --
        // per design.md, `resource_type` is deliberately not part of this
        // query's `WHERE` clause (it isn't part of the covering index
        // either), so it's accepted for interface symmetry but unused here.
        _resource_type: ResourceAuditResourceType,
        resource_id: Uuid,
    ) -> Result<FetchManyResponseKind<ResourceAuditLog>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
        })?;

        let records = resource_audit_log_model::table
            .filter(resource_audit_log_model::resource_id.eq(resource_id))
            .order_by(resource_audit_log_model::created_at.desc())
            .select(ResourceAuditLogModel::as_select())
            .load::<ResourceAuditLogModel>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to fetch resource audit log: {}",
                    e
                ))
            })?;

        if records.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let logs = records
            .into_iter()
            .map(parse_resource_audit_log_model)
            .collect::<Result<Vec<_>, String>>()
            .map_err(|e| fetching_err(e))?;

        Ok(FetchManyResponseKind::Found(logs))
    }

    #[tracing::instrument(name = "list_resource_audit_log_by_tenant", skip_all)]
    async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        page_size: i32,
        skip: i32,
    ) -> Result<FetchManyResponseKind<ResourceAuditLog>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
        })?;

        let base_query = resource_audit_log_model::table
            .filter(resource_audit_log_model::tenant_id.eq(tenant_id));

        let total = base_query
            .clone()
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to count resource audit log rows: {}",
                    e
                ))
            })?;

        let records = base_query
            .order_by(resource_audit_log_model::created_at.desc())
            .limit(page_size as i64)
            .offset(skip as i64)
            .select(ResourceAuditLogModel::as_select())
            .load::<ResourceAuditLogModel>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to fetch resource audit log: {}",
                    e
                ))
            })?;

        if records.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let logs = records
            .into_iter()
            .map(parse_resource_audit_log_model)
            .collect::<Result<Vec<_>, String>>()
            .map_err(|e| fetching_err(e))?;

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip as i64),
            size: Some(page_size as i64),
            records: logs,
        })
    }
}

fn parse_resource_audit_log_model(
    record: ResourceAuditLogModel,
) -> Result<ResourceAuditLog, String> {
    Ok(ResourceAuditLog {
        id: record.id,
        resource_type: resource_type_from_db_str(&record.resource_type)?,
        resource_id: record.resource_id,
        tenant_id: record.tenant_id,
        event: event_kind_from_db_str(&record.event)?,
        performed_by: serde_json::from_value(record.performed_by)
            .map_err(|e| format!("Failed to parse performed_by: {e}"))?,
        metadata: record.metadata,
        created_at: record.created_at.and_utc(),
    })
}
