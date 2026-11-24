use crate::domain::dtos::health_check_info::HealthCheckInfo;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

#[async_trait]
pub trait HealthCheckInfoWrite: Interface + Send + Sync {
    async fn register_health_check_info(
        &self,
        health_check_info: HealthCheckInfo,
    ) -> Result<(), MappedErrors>;

    async fn ensure_dailly_partition(
        &self,
        checked_at: DateTime<Local>,
    ) -> Result<(), MappedErrors>;
}
