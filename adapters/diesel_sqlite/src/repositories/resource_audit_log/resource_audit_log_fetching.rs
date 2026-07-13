use super::shared::{map_model_to_dto, resource_type_to_text};
use crate::{
    config::SqliteDbPoolProvider,
    models::resource_audit_log::ResourceAuditLog as ResourceAuditLogModel,
    schema::resource_audit_log, types::uuid_to_text,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        resource_audit_log::{ResourceAuditLog, ResourceAuditResourceType},
    },
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl ResourceAuditLogFetching for ResourceAuditLogFetchingSqlDbRepository {
    #[tracing::instrument(
        name = "list_resource_audit_log_by_resource",
        skip_all
    )]
    async fn list_by_resource(
        &self,
        resource_type: ResourceAuditResourceType,
        resource_id: Uuid,
    ) -> Result<FetchManyResponseKind<ResourceAuditLog>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let records = resource_audit_log::table
            .filter(
                resource_audit_log::resource_id.eq(uuid_to_text(&resource_id)),
            )
            .filter(
                resource_audit_log::resource_type
                    .eq(resource_type_to_text(&resource_type)),
            )
            .order(resource_audit_log::created_at.desc())
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
            .map(map_model_to_dto)
            .collect::<Result<Vec<_>, _>>()?;

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
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let tenant_id_text = uuid_to_text(&tenant_id);

        let total = resource_audit_log::table
            .filter(resource_audit_log::tenant_id.eq(&tenant_id_text))
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to count resource audit log: {}",
                    e
                ))
            })?;

        let records = resource_audit_log::table
            .filter(resource_audit_log::tenant_id.eq(&tenant_id_text))
            .order(resource_audit_log::created_at.desc())
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
            .map(map_model_to_dto)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip as i64),
            size: Some(page_size as i64),
            records: logs,
        })
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::setup_temp_db;
    use diesel::RunQueryDsl;
    use myc_core::domain::dtos::resource_audit_log::ResourceAuditEventKind;

    fn insert_row(
        conn: &mut diesel::SqliteConnection,
        resource_type: &str,
        resource_id: &str,
        tenant_id: Option<&str>,
        created_at: &str,
    ) {
        let row = ResourceAuditLogModel {
            id: Uuid::new_v4().to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            tenant_id: tenant_id.map(|value| value.to_string()),
            event: "created".to_string(),
            performed_by: serde_json::to_string(
                &myc_core::domain::dtos::written_by::WrittenBy::new_anemic(),
            )
            .unwrap(),
            metadata: "{}".to_string(),
            created_at: created_at.to_string(),
        };

        diesel::insert_into(resource_audit_log::table)
            .values(&row)
            .execute(conn)
            .unwrap();
    }

    #[tokio::test]
    async fn list_by_resource_filters_and_orders_by_created_at_desc(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let mut conn = db.provider.get_pool().get().unwrap();

        let target_resource_id = Uuid::new_v4().to_string();
        let other_resource_id = Uuid::new_v4().to_string();

        // Older row for the target resource.
        insert_row(
            &mut conn,
            "account",
            &target_resource_id,
            None,
            "2026-07-01T00:00:00+00:00",
        );
        // Newer row for the target resource -- must come first.
        insert_row(
            &mut conn,
            "account",
            &target_resource_id,
            None,
            "2026-07-10T00:00:00+00:00",
        );
        // Same resource_id, different resource_type -- must be excluded.
        insert_row(
            &mut conn,
            "webhook",
            &target_resource_id,
            None,
            "2026-07-11T00:00:00+00:00",
        );
        // Different resource entirely -- must be excluded.
        insert_row(
            &mut conn,
            "account",
            &other_resource_id,
            None,
            "2026-07-12T00:00:00+00:00",
        );
        drop(conn);

        let fetching = ResourceAuditLogFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let found = match fetching
            .list_by_resource(
                ResourceAuditResourceType::Account,
                Uuid::parse_str(&target_resource_id).unwrap(),
            )
            .await?
        {
            FetchManyResponseKind::Found(records) => records,
            other => panic!("expected Found, got {:?}", other),
        };

        assert_eq!(found.len(), 2);
        assert!(found[0].created_at > found[1].created_at);
        assert_eq!(found[0].event, ResourceAuditEventKind::Created);
        assert!(found
            .iter()
            .all(|record| record.resource_type
                == ResourceAuditResourceType::Account));

        Ok(())
    }

    #[tokio::test]
    async fn list_by_tenant_paginates_and_orders_by_created_at_desc(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let mut conn = db.provider.get_pool().get().unwrap();

        let tenant_id = Uuid::new_v4().to_string();
        let other_tenant_id = Uuid::new_v4().to_string();

        for day in 1..=3 {
            insert_row(
                &mut conn,
                "account",
                &Uuid::new_v4().to_string(),
                Some(&tenant_id),
                &format!("2026-07-0{day}T00:00:00+00:00"),
            );
        }
        // Different tenant -- must be excluded.
        insert_row(
            &mut conn,
            "account",
            &Uuid::new_v4().to_string(),
            Some(&other_tenant_id),
            "2026-07-09T00:00:00+00:00",
        );
        drop(conn);

        let fetching = ResourceAuditLogFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let (count, records) = match fetching
            .list_by_tenant(Uuid::parse_str(&tenant_id).unwrap(), 2, 0)
            .await?
        {
            FetchManyResponseKind::FoundPaginated {
                count, records, ..
            } => (count, records),
            other => panic!("expected FoundPaginated, got {:?}", other),
        };

        assert_eq!(count, 3);
        assert_eq!(records.len(), 2);
        assert!(records[0].created_at > records[1].created_at);

        let second_page = match fetching
            .list_by_tenant(Uuid::parse_str(&tenant_id).unwrap(), 2, 2)
            .await?
        {
            FetchManyResponseKind::FoundPaginated { records, .. } => records,
            other => panic!("expected FoundPaginated, got {:?}", other),
        };
        assert_eq!(second_page.len(), 1);

        Ok(())
    }
}
