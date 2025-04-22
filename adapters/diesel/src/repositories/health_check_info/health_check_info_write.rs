use crate::{
    models::{
        config::DbPoolProvider,
        health_check_info::HealthcheckInfo as HealthcheckInfoModel,
    },
    schema::healthcheck_logs as healthcheck_logs_model,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use diesel::{prelude::*, sql_query, sql_types::Bool};
use myc_core::domain::{
    dtos::{
        health_check_info::HealthCheckInfo,
        native_error_codes::NativeErrorCodes,
    },
    entities::HealthCheckInfoWrite,
};
use mycelium_base::utils::errors::{creation_err, MappedErrors};
use serde_json::to_value;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = HealthCheckInfoWrite)]
pub struct HealthCheckInfoWriteSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[derive(QueryableByName)]
struct PartitionExists {
    #[diesel(sql_type = Bool)]
    exists: bool,
}

#[async_trait]
impl HealthCheckInfoWrite for HealthCheckInfoWriteSqlDbRepository {
    #[tracing::instrument(name = "register_health_check_info", skip_all)]
    async fn register_health_check_info(
        &self,
        health_check_info: HealthCheckInfo,
    ) -> Result<(), MappedErrors> {
        let span = tracing::Span::current();

        span.record(
            "service_id",
            tracing::field::display(health_check_info.service_id),
        );
        span.record(
            "service_name",
            tracing::field::display(health_check_info.service_name.clone()),
        );
        span.record(
            "checked_at",
            tracing::field::display(health_check_info.checked_at),
        );
        span.record(
            "status_code",
            tracing::field::display(health_check_info.status_code),
        );

        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let health_check_info_model = HealthcheckInfoModel {
            service_id: health_check_info.service_id,
            service_name: health_check_info.service_name.clone(),
            checked_at: health_check_info.checked_at.into(),
            status_code: health_check_info.status_code as i32,
            response_time_ms: health_check_info.response_time_ms as i32,
            dns_resolved_ip: Some(health_check_info.dns_resolved_ip),
            response_body: health_check_info.response_body,
            error_message: health_check_info.error_message,
            headers: Some(to_value(health_check_info.headers).unwrap()),
            content_type: health_check_info.content_type,
            response_size_bytes: health_check_info
                .response_size_bytes
                .map(|b| b as i32),
            retry_count: health_check_info.retry_count.map(|c| c as i32),
            timeout_occurred: health_check_info.timeout_occurred,
        };

        diesel::insert_into(healthcheck_logs_model::table)
            .values(health_check_info_model)
            .execute(conn)
            .map_err(|err| {
                creation_err(format!(
                    "Failed to insert health check info: {err}",
                ))
            })?;

        tracing::trace!(
            "Health check info inserted for service {} with id {}",
            health_check_info.service_name,
            health_check_info.service_id
        );

        Ok(())
    }

    #[tracing::instrument(name = "ensure_dailly_partition", skip_all)]
    async fn ensure_dailly_partition(
        &self,
        checked_at: DateTime<Local>,
    ) -> Result<(), MappedErrors> {
        let span = tracing::Span::current();

        span.record("checked_at", tracing::field::display(checked_at));

        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let date = checked_at.date_naive();
        let partition_name =
            format!("healthcheck_logs_{}", date.format("%Y%m%d"));

        let check_sql = format!(
            "SELECT to_regclass('public.{}') IS NOT NULL AS exists",
            partition_name
        );

        let exists: bool = sql_query(check_sql)
            .load::<PartitionExists>(conn)
            .map_err(|err| {
                creation_err(format!(
                    "Failed to check if partition exists: {err}"
                ))
            })?
            .get(0)
            .map(|r| r.exists)
            .unwrap_or(false);

        span.record("exists", tracing::field::display(exists));

        if !exists {
            tracing::trace!(
                "Partition {} created for date {}",
                partition_name,
                date
            );

            let start_date = date.format("%Y-%m-%d").to_string();
            let end_date =
                (date.succ_opt().unwrap()).format("%Y-%m-%d").to_string();

            let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {partition} PARTITION OF healthcheck_logs \
            FOR VALUES FROM ('{start}') TO ('{end}')",
            partition = partition_name,
            start = start_date,
            end = end_date,
        );
            sql_query(create_sql).execute(conn).map_err(|err| {
                creation_err(format!("Failed to create partition: {err}"))
            })?;
        }

        Ok(())
    }
}
