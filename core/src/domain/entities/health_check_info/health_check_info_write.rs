use crate::domain::dtos::health_check_info::HealthCheckInfo;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};

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

impl Display for dyn HealthCheckInfoWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn HealthCheckInfoWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
